//! The one module that calls Win32 directly. Every `unsafe` block in the crate lives
//! here, each with the reason it is sound; callers get safe functions over plain data.
#![allow(unsafe_code)]

use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};
use windows_sys::Win32::System::Diagnostics::Debug::{SetThreadErrorMode, SEM_FAILCRITICALERRORS};
use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOVABLE};

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

    #[test]
    fn wide_strings_round_trip() {
        assert_eq!(from_wide(&wide("Ổ đĩa C:")), "Ổ đĩa C:");
    }
}
