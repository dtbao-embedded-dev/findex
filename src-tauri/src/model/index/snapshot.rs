//! A volume index saved to disk, so the next launch can search before any scan.
//!
//! Layout, little-endian: magic `FXI1`, format version, root, root FRN, the change
//! journal position the index is current to, then one record per live entry (FRN,
//! parent FRN, folder flag, UTF-8 name), and an FNV-1a hash of everything before it.
//! Written to a temp file and renamed, so a crash mid-save leaves the old snapshot.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::model::index::store::VolumeIndex;
use crate::model::win32::Journal;

const MAGIC: &[u8; 4] = b"FXI1";
const VERSION: u32 = 1;

pub struct Snapshot {
    pub index: VolumeIndex,
    /// The journal position the index was current to; `None` for a folder walk.
    pub journal: Option<Journal>,
}

/// Where the snapshot of `letter`'s drive lives under `dir`.
pub fn path_for(dir: &Path, letter: char) -> PathBuf {
    dir.join(format!("{letter}.fxi"))
}

pub fn save(path: &Path, index: &VolumeIndex, journal: Option<Journal>) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("fxi.tmp");
    {
        let mut out = Hashed::new(BufWriter::new(File::create(&temp)?));
        out.write_all(MAGIC)?;
        out.write_all(&VERSION.to_le_bytes())?;
        write_str(&mut out, index.root())?;
        out.write_all(&index.root_frn().to_le_bytes())?;
        match journal {
            Some(j) => {
                out.write_all(&[1])?;
                out.write_all(&j.id.to_le_bytes())?;
                out.write_all(&j.first_usn.to_le_bytes())?;
                out.write_all(&j.next_usn.to_le_bytes())?;
            }
            None => out.write_all(&[0])?,
        }
        out.write_all(&(index.len() as u64).to_le_bytes())?;
        for id in (0..index.node_count()).filter(|&id| index.is_live(id)) {
            out.write_all(&index.frn(id).to_le_bytes())?;
            out.write_all(&index.parent(id).to_le_bytes())?;
            out.write_all(&[u8::from(index.is_dir(id))])?;
            write_str(&mut out, index.name(id))?;
        }
        let hash = out.hash;
        let mut file = out.inner;
        file.write_all(&hash.to_le_bytes())?;
        file.into_inner()
            .map_err(io::IntoInnerError::into_error)?
            .sync_all()?;
    }
    std::fs::rename(&temp, path)?;
    Ok(())
}

/// The fewest bytes one entry takes: FRN, parent, folder flag, name length.
const MIN_ENTRY_BYTES: u64 = 8 + 8 + 1 + 4;

pub fn load(path: &Path) -> AppResult<Snapshot> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut input = Hashed::new(BufReader::with_capacity(1 << 20, file));
    let mut magic = [0_u8; 4];
    input.read_exact(&mut magic).map_err(corrupt)?;
    if &magic != MAGIC {
        return Err(AppError::Invalid("not a findex snapshot".into()));
    }
    let version = read_u32(&mut input)?;
    if version != VERSION {
        return Err(AppError::Invalid(format!(
            "snapshot version {version} is not supported"
        )));
    }
    let root = read_str(&mut input)?;
    let root_frn = read_u64(&mut input)?;
    let journal = match read_u8(&mut input)? {
        0 => None,
        1 => Some(Journal {
            id: read_u64(&mut input)?,
            first_usn: read_u64(&mut input)? as i64,
            next_usn: read_u64(&mut input)? as i64,
        }),
        _ => return Err(corrupt(io::ErrorKind::InvalidData.into())),
    };
    let count = read_u64(&mut input)?;
    let mut index = VolumeIndex::new(root, root_frn);
    // Sized from the file, never from the stored count alone: a damaged count must
    // not make the load reserve gigabytes.
    let fits = file_len / MIN_ENTRY_BYTES;
    let entries = count.min(fits) as usize;
    index.reserve(
        entries,
        file_len.saturating_sub(entries as u64 * MIN_ENTRY_BYTES) as usize,
    );
    for _ in 0..count {
        let frn = read_u64(&mut input)?;
        let parent = read_u64(&mut input)?;
        let is_dir = read_u8(&mut input)? != 0;
        let name = read_str(&mut input)?;
        index.upsert(frn, parent, &name, is_dir)?;
    }
    let expected = input.hash;
    let stored = read_u64(&mut input.inner)?;
    if stored != expected {
        return Err(AppError::Invalid("the snapshot is corrupt".into()));
    }
    Ok(Snapshot { index, journal })
}

fn write_str(out: &mut impl Write, text: &str) -> io::Result<()> {
    let len =
        u32::try_from(text.len()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    out.write_all(&len.to_le_bytes())?;
    out.write_all(text.as_bytes())
}

/// Longest name a snapshot may claim; NTFS names are at most 255 UTF-16 units, and
/// drive roots are short. A larger length means the file is damaged.
const MAX_STRING: u32 = 4096;

fn read_str(input: &mut impl Read) -> AppResult<String> {
    let len = read_u32(input)?;
    if len > MAX_STRING {
        return Err(corrupt(io::ErrorKind::InvalidData.into()));
    }
    let mut bytes = vec![0_u8; len as usize];
    input.read_exact(&mut bytes).map_err(corrupt)?;
    String::from_utf8(bytes).map_err(|_| corrupt(io::ErrorKind::InvalidData.into()))
}

fn read_u8(input: &mut impl Read) -> AppResult<u8> {
    let mut bytes = [0_u8; 1];
    input.read_exact(&mut bytes).map_err(corrupt)?;
    Ok(bytes[0])
}

fn read_u32(input: &mut impl Read) -> AppResult<u32> {
    let mut bytes = [0_u8; 4];
    input.read_exact(&mut bytes).map_err(corrupt)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(input: &mut impl Read) -> AppResult<u64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes).map_err(corrupt)?;
    Ok(u64::from_le_bytes(bytes))
}

/// A truncated or malformed file is reported as corrupt, not as an I/O failure.
fn corrupt(_: io::Error) -> AppError {
    AppError::Invalid("the snapshot is corrupt".into())
}

/// Passes bytes through while hashing them with 64-bit FNV-1a.
struct Hashed<T> {
    inner: T,
    hash: u64,
}

impl<T> Hashed<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            hash: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hash ^= u64::from(byte);
            self.hash = self.hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

impl<W: Write> Write for Hashed<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.feed(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<R: Read> Read for Hashed<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.feed(&buffer[..read]);
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: u64 = 5;

    /// A temp file removed when the test ends.
    struct TempFile(PathBuf);

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("fxi.tmp"));
        }
    }

    fn temp(name: &str) -> TempFile {
        TempFile(std::env::temp_dir().join(format!("findex-{name}-{}.fxi", std::process::id())))
    }

    fn sample() -> VolumeIndex {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        index.upsert(10, ROOT, "Users", true).unwrap();
        index.upsert(11, 10, "Ảnh cưới.jpg", false).unwrap();
        index.upsert(12, 10, "deleted.txt", false).unwrap();
        index.upsert(13, 10, "old name.txt", false).unwrap();
        index.remove(12);
        index.upsert(13, 10, "new name.txt", false).unwrap();
        index
    }

    fn paths(index: &VolumeIndex) -> Vec<String> {
        let mut paths: Vec<_> = (0..index.node_count())
            .filter(|&id| index.is_live(id))
            .map(|id| index.path(id))
            .collect();
        paths.sort();
        paths
    }

    const JOURNAL: Journal = Journal {
        id: 0xABCD,
        first_usn: 100,
        next_usn: 9_000,
    };

    #[test]
    fn round_trips_live_entries_and_the_journal_position() {
        let file = temp("round-trip");
        let index = sample();
        save(&file.0, &index, Some(JOURNAL)).unwrap();
        let loaded = load(&file.0).unwrap();
        assert_eq!(paths(&loaded.index), paths(&index));
        assert_eq!(loaded.index.len(), 3);
        assert_eq!(loaded.index.root_frn(), ROOT);
        assert_eq!(loaded.journal, Some(JOURNAL));
        assert!(!file.0.with_extension("fxi.tmp").exists());
    }

    /// Release-mode timing of a million-entry snapshot:
    /// `cargo test --release --lib -- --ignored perf_snapshot_one_million --nocapture`
    #[test]
    #[ignore]
    fn perf_snapshot_one_million() {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        for frn in 100..1_100_000_u64 {
            let parent = if frn % 10 == 0 { ROOT } else { frn - frn % 10 };
            let name = format!("Project_{}_report-{}.PDF", frn % 997, frn);
            index.upsert(frn, parent, &name, frn % 10 == 0).unwrap();
        }
        let file = temp("perf");
        let start = std::time::Instant::now();
        save(&file.0, &index, Some(JOURNAL)).unwrap();
        let saved = start.elapsed();
        let start = std::time::Instant::now();
        let loaded = load(&file.0).unwrap();
        let took = start.elapsed();
        let size = std::fs::metadata(&file.0).unwrap().len();
        println!(
            "1M entries: save {saved:?}, load {took:?}, {:.1} MB",
            size as f64 / 1e6
        );
        assert_eq!(loaded.index.len(), index.len());
    }

    #[test]
    fn a_walked_drive_has_no_journal() {
        let file = temp("no-journal");
        save(&file.0, &sample(), None).unwrap();
        assert_eq!(load(&file.0).unwrap().journal, None);
    }

    #[test]
    fn a_flipped_byte_is_caught_by_the_hash() {
        let file = temp("flipped");
        save(&file.0, &sample(), Some(JOURNAL)).unwrap();
        let mut bytes = std::fs::read(&file.0).unwrap();
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0x20;
        std::fs::write(&file.0, bytes).unwrap();
        assert!(matches!(load(&file.0), Err(AppError::Invalid(_))));
    }

    #[test]
    fn a_truncated_file_is_corrupt_not_a_panic() {
        let file = temp("truncated");
        save(&file.0, &sample(), Some(JOURNAL)).unwrap();
        let bytes = std::fs::read(&file.0).unwrap();
        for cut in [0, 3, 9, bytes.len() / 2, bytes.len() - 1] {
            std::fs::write(&file.0, &bytes[..cut]).unwrap();
            assert!(
                matches!(load(&file.0), Err(AppError::Invalid(_))),
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn another_version_is_refused() {
        let file = temp("version");
        save(&file.0, &sample(), None).unwrap();
        let mut bytes = std::fs::read(&file.0).unwrap();
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        std::fs::write(&file.0, bytes).unwrap();
        let Err(AppError::Invalid(message)) = load(&file.0) else {
            panic!("version 2 was accepted");
        };
        assert!(message.contains("version 2"));
    }

    #[test]
    fn a_missing_snapshot_is_an_io_error() {
        let missing = std::env::temp_dir().join("findex-snapshot-that-does-not-exist.fxi");
        assert!(matches!(load(&missing), Err(AppError::Io(_))));
    }
}
