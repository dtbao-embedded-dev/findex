//! The one module that calls Win32 directly. Every `unsafe` block in the crate lives
//! here, each with the reason it is sound; callers get safe functions over plain data.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::io;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_HANDLE_EOF, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Diagnostics::Debug::{SetThreadErrorMode, SEM_FAILCRITICALERRORS};
use windows_sys::Win32::System::Ioctl::{
    FSCTL_ENUM_USN_DATA, FSCTL_QUERY_USN_JOURNAL, MFT_ENUM_DATA_V1, USN_JOURNAL_DATA_V0,
};
use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOVABLE};
use windows_sys::Win32::System::IO::DeviceIoControl;

/// A mounted local volume, as Windows reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub letter: char,
    pub label: String,
    pub filesystem: String,
    pub removable: bool,
    pub total: u64,
    pub free: u64,
}

/// Every local volume with media in it. Network and optical drives are left out: a
/// disconnected network drive can block for seconds, and neither can be indexed fast.
pub fn list_volumes() -> Vec<Volume> {
    let _quiet = QuietErrors::new();
    // SAFETY: takes no arguments and only returns a bit mask.
    let mask = unsafe { GetLogicalDrives() };
    (0..26_u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .filter_map(|bit| volume(char::from(b'A' + bit)))
        .collect()
}

fn volume(letter: char) -> Option<Volume> {
    let root = wide(&format!("{letter}:\\"));
    // SAFETY: `root` is a NUL-terminated UTF-16 string that outlives the call.
    let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
    if !matches!(kind, DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_RAMDISK) {
        return None;
    }
    let mut label = [0_u16; 261];
    let mut filesystem = [0_u16; 261];
    // SAFETY: both buffers are writable and their lengths are passed with them; the
    // optional out-pointers are null, which the API allows.
    let ok = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    if ok == 0 {
        // No media, as in an empty card reader.
        return None;
    }
    let (mut free, mut total) = (0_u64, 0_u64);
    // SAFETY: `root` is NUL-terminated; the out-pointers point at live u64 values and
    // the third is null, which the API allows.
    let sized =
        unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
    if sized == 0 {
        (free, total) = (0, 0);
    }
    Some(Volume {
        letter,
        label: from_wide(&label),
        filesystem: from_wide(&filesystem),
        removable: kind == DRIVE_REMOVABLE,
        total,
        free,
    })
}

/// Turns off the "insert a disk" dialog for this thread while it lives, so probing
/// an empty drive fails quietly instead of blocking on a modal box.
struct QuietErrors(u32);

impl QuietErrors {
    fn new() -> Self {
        let mut old = 0;
        // SAFETY: `old` is a live u32 the API writes the previous mode into.
        unsafe { SetThreadErrorMode(SEM_FAILCRITICALERRORS, &mut old) };
        Self(old)
    }
}

impl Drop for QuietErrors {
    fn drop(&mut self) {
        // SAFETY: restores the mode saved in `new`; the old-mode pointer may be null.
        unsafe { SetThreadErrorMode(self.0, std::ptr::null_mut()) };
    }
}

/// An open handle to a whole volume, such as `\\.\C:`, closed on drop.
pub struct VolumeHandle(HANDLE);

impl Drop for VolumeHandle {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful CreateFileW and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// Opens a volume for reading its file table. Needs administrator rights: without
/// them this fails with access denied, and callers fall back to a folder walk.
pub fn open_volume(letter: char) -> io::Result<VolumeHandle> {
    let path = wide(&format!(r"\\.\{letter}:"));
    // SAFETY: `path` is NUL-terminated and outlives the call; the security attributes
    // and template handle may be null.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(VolumeHandle(handle))
}

/// Where a volume's change journal stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Journal {
    pub id: u64,
    pub first_usn: i64,
    pub next_usn: i64,
}

pub fn query_journal(volume: &VolumeHandle) -> io::Result<Journal> {
    let mut out = [0_u64; std::mem::size_of::<USN_JOURNAL_DATA_V0>() / 8];
    ioctl(volume, FSCTL_QUERY_USN_JOURNAL, &(), as_bytes_mut(&mut out))?;
    // SAFETY: `out` is exactly the size of USN_JOURNAL_DATA_V0 and 8-byte aligned,
    // and every bit pattern is a valid value of that plain-integer struct.
    let data = unsafe { std::ptr::read(out.as_ptr().cast::<USN_JOURNAL_DATA_V0>()) };
    Ok(Journal {
        id: data.UsnJournalID,
        first_usn: data.FirstUsn,
        next_usn: data.NextUsn,
    })
}

/// One file or folder as the file table or the change journal reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsnRecord {
    pub frn: u64,
    pub parent: u64,
    pub usn: i64,
    pub reason: u32,
    pub attributes: u32,
    pub name: String,
}

impl UsnRecord {
    pub fn is_dir(&self) -> bool {
        self.attributes & FILE_ATTRIBUTE_DIRECTORY != 0
    }
}

/// Bytes per file-table read; 1 MiB holds several thousand records.
const ENUM_BUFFER: usize = 1 << 20;

/// Calls `each` for every file and folder in the volume's file table, as of
/// `high_usn`, in file-table order (a child can come before its parent).
pub fn enum_mft(
    volume: &VolumeHandle,
    high_usn: i64,
    mut each: impl FnMut(UsnRecord),
) -> io::Result<()> {
    let mut input = MFT_ENUM_DATA_V1 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: high_usn,
        // Version 2 records only: 64-bit file references, the NTFS layout.
        MinMajorVersion: 2,
        MaxMajorVersion: 2,
    };
    let mut buffer = vec![0_u64; ENUM_BUFFER / 8];
    loop {
        let out = as_bytes_mut(&mut buffer);
        let filled = match ioctl(volume, FSCTL_ENUM_USN_DATA, &input, out) {
            Ok(filled) => filled,
            Err(err) if err.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) => return Ok(()),
            Err(err) => return Err(err),
        };
        if filled <= 8 {
            return Ok(());
        }
        // The output starts with the reference number to continue from.
        input.StartFileReferenceNumber = read_u64(out, 0).unwrap_or(u64::MAX);
        parse_usn_records(&out[8..filled])
            .into_iter()
            .for_each(&mut each);
    }
}

/// Reads packed USN_RECORD_V2 entries. Other record versions are skipped; a record
/// that does not fit the buffer ends the parse instead of reading past it.
pub fn parse_usn_records(buffer: &[u8]) -> Vec<UsnRecord> {
    let mut records = Vec::new();
    let mut at = 0;
    while let Some(length) = read_u32(buffer, at).map(|n| n as usize) {
        if length < 60 || at + length > buffer.len() {
            break;
        }
        let record = &buffer[at..at + length];
        if read_u16(record, 4) == Some(2) {
            if let Some(parsed) = parse_v2(record) {
                records.push(parsed);
            }
        }
        at += length;
    }
    records
}

fn parse_v2(record: &[u8]) -> Option<UsnRecord> {
    let name_len = usize::from(read_u16(record, 56)?);
    let name_off = usize::from(read_u16(record, 58)?);
    let name_bytes = record.get(name_off..name_off + name_len)?;
    let units: Vec<u16> = name_bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Some(UsnRecord {
        frn: read_u64(record, 8)?,
        parent: read_u64(record, 16)?,
        usn: read_u64(record, 24)? as i64,
        reason: read_u32(record, 40)?,
        attributes: read_u32(record, 52)?,
        name: String::from_utf16_lossy(&units),
    })
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn read_u64(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// One DeviceIoControl call; returns how many output bytes were written.
fn ioctl<I>(volume: &VolumeHandle, code: u32, input: &I, out: &mut [u8]) -> io::Result<usize> {
    let mut written = 0_u32;
    // SAFETY: `input` and `out` are live for the call and their sizes are passed with
    // them; the call is synchronous, so no OVERLAPPED is needed.
    let ok = unsafe {
        DeviceIoControl(
            volume.0,
            code,
            (input as *const I).cast::<c_void>(),
            std::mem::size_of::<I>() as u32,
            out.as_mut_ptr().cast::<c_void>(),
            out.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(written as usize)
}

/// Views a u64 buffer as bytes: u64 storage gives the 8-byte alignment that file
/// table records are laid out with.
fn as_bytes_mut(words: &mut [u64]) -> &mut [u8] {
    // SAFETY: u8 has no alignment or validity requirements, and the byte length is
    // exactly the size of the borrowed u64 slice, which stays borrowed meanwhile.
    unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), words.len() * 8) }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_system_drive_with_its_filesystem_and_size() {
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let letter = system.chars().next().unwrap();
        let volumes = list_volumes();
        let volume = volumes.iter().find(|v| v.letter == letter).unwrap();
        assert_eq!(volume.filesystem, "NTFS");
        assert!(volume.total > 0 && volume.free <= volume.total);
    }

    /// Builds one USN_RECORD_V2 the way the kernel lays it out.
    fn record_v2(frn: u64, parent: u64, attributes: u32, name: &str) -> Vec<u8> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let length = (60 + units.len() * 2).div_ceil(8) * 8;
        let mut out = vec![0_u8; length];
        out[0..4].copy_from_slice(&(length as u32).to_le_bytes());
        out[4..6].copy_from_slice(&2_u16.to_le_bytes());
        out[8..16].copy_from_slice(&frn.to_le_bytes());
        out[16..24].copy_from_slice(&parent.to_le_bytes());
        out[24..32].copy_from_slice(&77_i64.to_le_bytes());
        out[40..44].copy_from_slice(&0x100_u32.to_le_bytes());
        out[52..56].copy_from_slice(&attributes.to_le_bytes());
        out[56..58].copy_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        out[58..60].copy_from_slice(&60_u16.to_le_bytes());
        for (i, unit) in units.iter().enumerate() {
            out[60 + i * 2..62 + i * 2].copy_from_slice(&unit.to_le_bytes());
        }
        out
    }

    #[test]
    fn parses_packed_v2_records() {
        let dir = 0x0001_0000_0000_0040;
        let mut buffer = record_v2(dir, 5, FILE_ATTRIBUTE_DIRECTORY, "Ảnh");
        buffer.extend(record_v2(0x0002_0000_0000_0041, dir, 0x20, "a.txt"));
        let records = parse_usn_records(&buffer);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "Ảnh");
        assert!(records[0].is_dir());
        assert_eq!(records[0].usn, 77);
        assert_eq!(records[0].reason, 0x100);
        assert_eq!(records[1].parent, dir);
        assert!(!records[1].is_dir());
    }

    #[test]
    fn skips_other_versions_and_stops_at_a_truncated_record() {
        let mut buffer = record_v2(10, 5, 0, "v3.txt");
        buffer[4..6].copy_from_slice(&3_u16.to_le_bytes());
        buffer.extend(record_v2(11, 5, 0, "kept.txt"));
        let cut = record_v2(12, 5, 0, "cut.txt");
        buffer.extend(&cut[..cut.len() - 8]);
        let names: Vec<_> = parse_usn_records(&buffer)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, ["kept.txt"]);
    }

    #[test]
    fn a_zero_length_record_ends_the_parse() {
        assert!(parse_usn_records(&[0_u8; 64]).is_empty());
    }

    #[test]
    fn wide_strings_round_trip() {
        assert_eq!(from_wide(&wide("Ổ đĩa C:")), "Ổ đĩa C:");
    }
}
