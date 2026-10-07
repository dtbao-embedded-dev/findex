//! The one module that calls Win32 directly. Every `unsafe` block in the crate lives
//! here, each with the reason it is sound; callers get safe functions over plain data.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::io;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_HANDLE_EOF, ERROR_JOURNAL_DELETE_IN_PROGRESS, ERROR_JOURNAL_ENTRY_DELETED,
    ERROR_JOURNAL_NOT_ACTIVE, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::System::Diagnostics::Debug::{SetThreadErrorMode, SEM_FAILCRITICALERRORS};
use windows_sys::Win32::System::Ioctl::{
    FSCTL_ENUM_USN_DATA, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, MFT_ENUM_DATA_V1,
    READ_USN_JOURNAL_DATA_V1, USN_JOURNAL_DATA_V0,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess,
    InitializeProcThreadAttributeList, OpenProcess, OpenProcessToken, UpdateProcThreadAttribute,
    CREATE_NEW_CONSOLE, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_CREATE_PROCESS, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, STARTUPINFOEXW,
};
use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOVABLE};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::UI::Shell::{
    SHGetFileInfoW, ShellExecuteW, SHFILEINFOW, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetShellWindow, GetWindowThreadProcessId, SW_SHOWNORMAL,
};

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
/// `high_usn`, in file-table order (a child can come before its parent). Stops early
/// when `each` returns false.
pub fn enum_mft(
    volume: &VolumeHandle,
    high_usn: i64,
    mut each: impl FnMut(UsnRecord) -> bool,
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
        for record in parse_usn_records(&out[8..filled]) {
            if !each(record) {
                return Ok(());
            }
        }
    }
}

/// Changes read from a journal, and where the next read starts.
pub struct UsnBatch {
    pub next_usn: i64,
    pub records: Vec<UsnRecord>,
}

/// Reads the journal entries written since `start`, without waiting: an empty batch
/// means nothing changed. `buffer` is reused between calls.
pub fn read_usn(
    volume: &VolumeHandle,
    journal_id: u64,
    start: i64,
    buffer: &mut [u64],
) -> io::Result<UsnBatch> {
    let input = READ_USN_JOURNAL_DATA_V1 {
        StartUsn: start,
        ReasonMask: u32::MAX,
        ReturnOnlyOnClose: 0,
        Timeout: 0,
        BytesToWaitFor: 0,
        UsnJournalID: journal_id,
        MinMajorVersion: 2,
        MaxMajorVersion: 2,
    };
    let out = as_bytes_mut(buffer);
    let filled = ioctl(volume, FSCTL_READ_USN_JOURNAL, &input, out)?;
    // Fewer than 8 bytes never happens on success; if it did, the buffer would still
    // hold the previous call's position, so keep `start` instead.
    let next_usn = if filled >= 8 {
        read_u64(out, 0).map_or(start, |usn| usn as i64)
    } else {
        start
    };
    let records = parse_usn_records(out.get(8..filled).unwrap_or_default());
    Ok(UsnBatch { next_usn, records })
}

/// True when the journal no longer holds the entries asked for: it wrapped, was
/// deleted or was recreated. The index must then be rebuilt from the file table.
pub fn is_journal_gap(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error().map(|code| code as u32),
        Some(
            ERROR_JOURNAL_ENTRY_DELETED
                | ERROR_JOURNAL_NOT_ACTIVE
                | ERROR_JOURNAL_DELETE_IN_PROGRESS
        )
    )
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

/// True when this process runs with administrator rights (UAC elevated).
pub fn is_elevated() -> bool {
    elevation() == Some(true)
}

/// Whether this process is elevated; `None` when its token cannot be read, so a
/// caller can treat "unknown" as the safe answer for its own purpose.
pub fn elevation() -> Option<bool> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing, and
    // `token` is a live out-pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut written = 0_u32;
    // SAFETY: the buffer is one TOKEN_ELEVATION and its exact size is passed with it;
    // `token` came from a successful OpenProcessToken and is closed once, below.
    let ok = unsafe {
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast::<c_void>(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut written,
        );
        CloseHandle(token);
        ok
    };
    (ok != 0).then_some(elevation.TokenIsElevated != 0)
}

/// Starts `exe` with `args` again through the UAC prompt. False when the user said
/// No or the start failed; the caller then carries on without the rights.
pub fn run_elevated(exe: &std::path::Path, args: &str) -> bool {
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let params = wide(args);
    // SAFETY: every string is NUL-terminated and outlives the call; the window and
    // working-directory arguments may be null.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute reports success as a value above 32.
    result as isize > 32
}

/// Explorer's name for a type, such as "PDF Document": for files ending in `.ext`,
/// or for folders when `ext` is `None`. Asks the shell's registry of types without
/// touching the disk; `None` when the shell has no name for it.
pub fn file_type_name(ext: Option<&str>) -> Option<String> {
    let (name, attributes) = match ext {
        Some(ext) => (format!("file.{ext}"), FILE_ATTRIBUTE_NORMAL),
        None => ("folder".to_owned(), FILE_ATTRIBUTE_DIRECTORY),
    };
    let name = wide(&name);
    // SAFETY: the reserved pointer must be null. Any success (S_OK, or S_FALSE when
    // this thread was already initialised) is balanced by CoUninitialize below.
    let com = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    // SAFETY: a plain C struct whose all-zero value is valid.
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    // SAFETY: `name` is NUL-terminated and outlives the call; `info` is one
    // SHFILEINFOW and its exact size is passed with it.
    let ok = unsafe {
        SHGetFileInfoW(
            name.as_ptr(),
            attributes,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
        )
    };
    if com >= 0 {
        // SAFETY: balances the successful CoInitializeEx above, on the same thread.
        unsafe { CoUninitialize() };
    }
    let text = from_wide(&info.szTypeName);
    (ok != 0 && !text.is_empty()).then_some(text)
}

/// A process or thread handle, closed on drop.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful Win32 call and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// An initialised attribute list, deleted on drop; the buffer behind it must outlive it.
struct AttributeList(LPPROC_THREAD_ATTRIBUTE_LIST);

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: the list was initialised by InitializeProcThreadAttributeList and is
        // deleted once, before its buffer is freed.
        unsafe { DeleteProcThreadAttributeList(self.0) };
    }
}

/// Starts `command_line` in a new console, working in `cwd`, with the environment
/// block `env`: UTF-16 `name=value` strings, each NUL-terminated, then one more NUL;
/// empty means this process's own. With `as_shell_user`,
/// Explorer becomes its parent, and a process takes its parent's token: it then
/// runs with the signed-in user's normal rights even when Findex is elevated. That
/// fails when Explorer is not running.
pub fn create_process(
    command_line: &str,
    cwd: &std::path::Path,
    as_shell_user: bool,
    env: &[u16],
) -> io::Result<()> {
    let mut parent: Option<Owned> = None;
    let mut buffer: Vec<u64> = Vec::new();
    let mut list: Option<AttributeList> = None;
    if as_shell_user {
        // SAFETY: takes no arguments; a null window means no shell is running.
        let shell = unsafe { GetShellWindow() };
        if shell.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Explorer is not running",
            ));
        }
        let mut pid = 0_u32;
        // SAFETY: `shell` is a window handle just returned; `pid` is a live out-pointer.
        unsafe { GetWindowThreadProcessId(shell, &mut pid) };
        if pid == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a plain call; the null failure value is checked below.
        let handle = unsafe { OpenProcess(PROCESS_CREATE_PROCESS, 0, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let parent = parent.insert(Owned(handle));

        let mut size = 0_usize;
        // SAFETY: a null list asks only for the size; this call fails by design.
        unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size) };
        // u64 words keep the list 8-byte aligned.
        buffer.resize(size.div_ceil(8), 0);
        let raw: LPPROC_THREAD_ATTRIBUTE_LIST = buffer.as_mut_ptr().cast();
        // SAFETY: `buffer` holds at least `size` bytes and is not touched again until
        // the list is deleted, which happens first because `list` drops before it.
        if unsafe { InitializeProcThreadAttributeList(raw, 1, 0, &mut size) } == 0 {
            return Err(io::Error::last_os_error());
        }
        list = Some(AttributeList(raw));
        // SAFETY: the value points at the parent handle, which stays open, unmoved,
        // until after CreateProcessW below has read it.
        let ok = unsafe {
            UpdateProcThreadAttribute(
                raw,
                0,
                PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
                (&parent.0 as *const HANDLE).cast::<c_void>(),
                std::mem::size_of::<HANDLE>(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
    }

    // SAFETY: both are plain C structs whose all-zero value is valid (null pointers).
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut flags = CREATE_NEW_CONSOLE;
    if !env.is_empty() {
        flags |= CREATE_UNICODE_ENVIRONMENT;
    }
    match &list {
        Some(list) => {
            startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
            startup.lpAttributeList = list.0;
            flags |= EXTENDED_STARTUPINFO_PRESENT;
        }
        None => startup.StartupInfo.cb = std::mem::size_of_val(&startup.StartupInfo) as u32,
    }
    // CreateProcessW may write into the command line, so it gets its own buffer.
    let mut command = wide(command_line);
    let dir = wide(&cwd.to_string_lossy());
    // SAFETY: the strings are NUL-terminated and outlive the call; `env`, when not
    // empty, is a UTF-16 block ending in two NULs, as CREATE_UNICODE_ENVIRONMENT
    // requires, and outlives the call; the startup info
    // is a STARTUPINFOEXW whenever EXTENDED_STARTUPINFO_PRESENT is set, and its `cb`
    // says which; the attribute list and parent handle are alive until after it.
    let ok = unsafe {
        CreateProcessW(
            std::ptr::null(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            if env.is_empty() {
                std::ptr::null()
            } else {
                env.as_ptr().cast::<c_void>()
            },
            dir.as_ptr(),
            (&startup as *const STARTUPINFOEXW).cast(),
            &mut process,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    drop(Owned(process.hProcess));
    drop(Owned(process.hThread));
    drop(list);
    drop(buffer);
    drop(parent);
    Ok(())
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

    /// Needs a desktop session with Explorer running, and flashes a console window.
    #[test]
    #[ignore]
    fn a_process_started_as_the_shell_user_works_in_the_given_folder() {
        let dir = std::env::temp_dir().join(format!("findex spawn;{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        create_process("cmd.exe /c cd > where.txt", &dir, true, &[]).unwrap();
        let written = dir.join("where.txt");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !written.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let text = std::fs::read_to_string(&written).unwrap();
        assert_eq!(text.trim(), dir.display().to_string());
        std::fs::remove_dir_all(dir).unwrap();
    }

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
