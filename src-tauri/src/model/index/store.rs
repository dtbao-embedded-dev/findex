//! One volume's name index. Each file and folder is a node holding its own name and
//! its parent's file reference number (FRN), so a full path is rebuilt only for the
//! results shown, never stored per entry. A move or rename touches one node.
//!
//! Lowercased names sit end to end in one arena, each followed by a `\0`, so a query
//! can run one SIMD substring search over the whole volume instead of one per name.

use std::collections::HashMap;

use crate::error::{AppError, AppResult};

/// Where a string sits in an arena.
#[derive(Clone, Copy, Debug)]
struct Span {
    off: u32,
    len: u32,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    parent: u64,
    name: Span,
    lower: Span,
    is_dir: bool,
    deleted: bool,
}

/// The separator after every name in the lowercase arena; Windows forbids it in names.
pub const SEPARATOR: u8 = 0;

/// Paths deeper than this are treated as a parent cycle and cut short.
const MAX_DEPTH: usize = 512;

pub struct VolumeIndex {
    /// The volume's root as shown in a path, for example `C:\`.
    root: String,
    /// The FRN that stands for the root folder; a parent equal to it ends a path.
    root_frn: u64,
    nodes: Vec<Node>,
    names: String,
    lower: String,
    /// Start offset of every string ever written to `lower`, with its node. Offsets
    /// only grow, so the list stays sorted and a match position finds its owner by
    /// binary search. A renamed node leaves its old entry behind as a dead string.
    starts: Vec<(u32, u32)>,
    by_frn: HashMap<u64, u32>,
    frns: Vec<u64>,
    live: usize,
    dead_bytes: usize,
}

impl VolumeIndex {
    pub fn new(root: impl Into<String>, root_frn: u64) -> Self {
        Self {
            root: root.into(),
            root_frn,
            nodes: Vec::new(),
            names: String::new(),
            lower: String::new(),
            starts: Vec::new(),
            by_frn: HashMap::new(),
            frns: Vec::new(),
            live: 0,
            dead_bytes: 0,
        }
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn root_frn(&self) -> u64 {
        self.root_frn
    }

    /// Live entries: files and folders that exist.
    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Adds an entry, or renames and moves it when its FRN is already known.
    pub fn upsert(&mut self, frn: u64, parent: u64, name: &str, is_dir: bool) -> AppResult<()> {
        let (name_span, lower_span) = self.push_strings(name)?;
        match self.by_frn.get(&frn) {
            Some(&id) => {
                let node = &mut self.nodes[id as usize];
                self.dead_bytes += (node.name.len + node.lower.len + 1) as usize;
                node.parent = parent;
                node.name = name_span;
                node.lower = lower_span;
                node.is_dir = is_dir;
                self.starts.push((lower_span.off, id));
            }
            None => {
                let id = u32::try_from(self.nodes.len()).map_err(|_| too_large())?;
                self.nodes.push(Node {
                    parent,
                    name: name_span,
                    lower: lower_span,
                    is_dir,
                    deleted: false,
                });
                self.frns.push(frn);
                self.by_frn.insert(frn, id);
                self.starts.push((lower_span.off, id));
                self.live += 1;
            }
        }
        Ok(())
    }

    /// Marks an entry deleted. Returns false when the FRN was not indexed.
    pub fn remove(&mut self, frn: u64) -> bool {
        let Some(id) = self.by_frn.remove(&frn) else {
            return false;
        };
        let node = &mut self.nodes[id as usize];
        node.deleted = true;
        self.dead_bytes += (node.name.len + node.lower.len + 1) as usize;
        self.live -= 1;
        true
    }

    /// The node id of an FRN, when it is indexed.
    pub fn node_of(&self, frn: u64) -> Option<u32> {
        self.by_frn.get(&frn).copied()
    }

    /// Rebuilds a full path by walking parent links up to the root.
    pub fn path(&self, id: u32) -> String {
        let mut parts = vec![self.name(id)];
        let mut parent = self.nodes[id as usize].parent;
        while parent != self.root_frn && parts.len() < MAX_DEPTH {
            match self.by_frn.get(&parent) {
                Some(&up) => {
                    parts.push(self.name(up));
                    parent = self.nodes[up as usize].parent;
                }
                None => break,
            }
        }
        let mut path = self.root.clone();
        for (i, part) in parts.iter().rev().enumerate() {
            if i > 0 {
                path.push('\\');
            }
            path.push_str(part);
        }
        path
    }

    /// Drops deleted nodes and dead strings once they outweigh the live ones.
    pub fn compact_if_wasteful(&mut self) -> AppResult<()> {
        if self.dead_bytes > self.lower.len() / 2 + (1 << 20) {
            self.compact()?;
        }
        Ok(())
    }

    /// Rewrites the arenas with live entries only, renumbering nodes.
    pub fn compact(&mut self) -> AppResult<()> {
        let mut fresh = VolumeIndex::new(self.root.clone(), self.root_frn);
        for (id, node) in self.nodes.iter().enumerate() {
            if !node.deleted {
                fresh.upsert(
                    self.frns[id],
                    node.parent,
                    self.name(id as u32),
                    node.is_dir,
                )?;
            }
        }
        *self = fresh;
        Ok(())
    }

    /// The name with its original case.
    pub fn name(&self, id: u32) -> &str {
        let span = self.nodes[id as usize].name;
        &self.names[span.off as usize..(span.off + span.len) as usize]
    }

    /// The lowercase name the query matches against.
    pub fn lower_name(&self, id: u32) -> &str {
        let span = self.nodes[id as usize].lower;
        &self.lower[span.off as usize..(span.off + span.len) as usize]
    }

    pub fn is_dir(&self, id: u32) -> bool {
        self.nodes[id as usize].is_dir
    }

    /// The parent FRN of a node.
    pub fn parent(&self, id: u32) -> u64 {
        self.nodes[id as usize].parent
    }

    pub fn frn(&self, id: u32) -> u64 {
        self.frns[id as usize]
    }

    /// Every node id, deleted ones included; `is_live` tells them apart.
    pub fn node_count(&self) -> u32 {
        // upsert refuses a node past u32::MAX, so this never truncates.
        self.nodes.len() as u32
    }

    pub fn is_live(&self, id: u32) -> bool {
        !self.nodes[id as usize].deleted
    }

    /// The lowercase arena: names separated by `SEPARATOR`.
    pub fn lower_arena(&self) -> &[u8] {
        self.lower.as_bytes()
    }

    /// The node whose current lowercase name contains arena byte `at`, if any. A match
    /// inside a dead string (an old name, or a deleted entry) has no owner.
    pub fn owner_of(&self, at: usize) -> Option<u32> {
        let slot = self
            .starts
            .partition_point(|&(off, _)| off as usize <= at)
            .checked_sub(1)?;
        let (off, id) = self.starts[slot];
        let node = &self.nodes[id as usize];
        (!node.deleted && node.lower.off == off).then_some(id)
    }

    fn push_strings(&mut self, name: &str) -> AppResult<(Span, Span)> {
        let name_span = push(&mut self.names, name)?;
        let lower_span = push(&mut self.lower, &name.to_lowercase())?;
        self.lower.push(SEPARATOR as char);
        Ok((name_span, lower_span))
    }
}

fn push(arena: &mut String, text: &str) -> AppResult<Span> {
    let off = u32::try_from(arena.len()).map_err(|_| too_large())?;
    let len = u32::try_from(text.len()).map_err(|_| too_large())?;
    off.checked_add(len).ok_or_else(too_large)?;
    arena.push_str(text);
    Ok(Span { off, len })
}

fn too_large() -> AppError {
    AppError::Invalid("the index of one drive is larger than 4 GiB".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: u64 = 5;

    fn sample() -> VolumeIndex {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        index.upsert(10, ROOT, "Users", true).unwrap();
        index.upsert(11, 10, "Ada", true).unwrap();
        index.upsert(12, 11, "Report.PDF", false).unwrap();
        index
    }

    #[test]
    fn rebuilds_a_full_path_from_parent_links() {
        let index = sample();
        let id = index.node_of(12).unwrap();
        assert_eq!(index.path(id), "C:\\Users\\Ada\\Report.PDF");
        assert_eq!(index.lower_name(id), "report.pdf");
    }

    #[test]
    fn a_child_indexed_before_its_parent_still_resolves() {
        let mut index = VolumeIndex::new("D:\\", ROOT);
        index.upsert(21, 20, "late.txt", false).unwrap();
        index.upsert(20, ROOT, "Docs", true).unwrap();
        assert_eq!(index.path(index.node_of(21).unwrap()), "D:\\Docs\\late.txt");
    }

    #[test]
    fn a_rename_and_move_keeps_one_live_entry() {
        let mut index = sample();
        index.upsert(13, ROOT, "Archive", true).unwrap();
        index.upsert(12, 13, "Final.pdf", false).unwrap();
        let id = index.node_of(12).unwrap();
        assert_eq!(index.len(), 4);
        assert_eq!(index.path(id), "C:\\Archive\\Final.pdf");
        // The old name's bytes are still in the arena but no longer owned.
        let old = index.lower.find("report.pdf").unwrap();
        assert_eq!(index.owner_of(old), None);
        let new = index.lower.find("final.pdf").unwrap();
        assert_eq!(index.owner_of(new), Some(id));
    }

    #[test]
    fn a_removed_entry_is_no_longer_owned_or_counted() {
        let mut index = sample();
        assert!(index.remove(12));
        assert!(!index.remove(12));
        assert_eq!(index.len(), 2);
        let at = index.lower.find("report").unwrap();
        assert_eq!(index.owner_of(at), None);
    }

    #[test]
    fn compaction_drops_dead_entries_and_keeps_paths() {
        let mut index = sample();
        index.upsert(12, 11, "Renamed.pdf", false).unwrap();
        index.remove(10);
        index.upsert(10, ROOT, "Home", true).unwrap();
        index.compact().unwrap();
        assert_eq!(index.node_count(), 3);
        assert!(!index.lower.contains("report"));
        let id = index.node_of(12).unwrap();
        assert_eq!(index.path(id), "C:\\Home\\Ada\\Renamed.pdf");
    }

    #[test]
    fn a_parent_cycle_ends_instead_of_looping() {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        index.upsert(30, 31, "a", true).unwrap();
        index.upsert(31, 30, "b", true).unwrap();
        let path = index.path(index.node_of(30).unwrap());
        assert!(path.starts_with("C:\\"));
    }
}
