//! Atomic file-publish primitives.
//!
//! Publishing is atomic: render into a fresh `out-<ts>/` snapshot, then flip a
//! `current` symlink to it with an atomic rename. A reader pointed at `current/`
//! always sees a complete tree, never a half-written one. Old snapshots are
//! garbage-collected, keeping the newest `keep` plus whatever `current` resolves
//! to (never delete what's being served).
//!
//! Consumers either call the [`publish`] convenience with a prebuilt file map, or
//! drive the primitives themselves ([`new_snapshot`] -> render into it ->
//! [`flip_current`] -> [`gc`]) when they render directly into the snapshot dir.

use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One file to write into a snapshot: `(path relative to the snapshot root,
/// bytes)`. Parent directories are created as needed.
pub type TreeFile = (String, Vec<u8>);

/// Create a fresh `out-<ts>/` snapshot directory under `out` (creating `out`
/// itself if needed) and return its path. The nanosecond timestamp name sorts
/// chronologically.
pub fn new_snapshot(out: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(out)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(e.to_string()))?
        .as_nanos();
    let snap = out.join(format!("out-{ts}"));
    fs::create_dir_all(&snap)?;
    Ok(snap)
}

/// Write every file in the map into `dir`, creating parent directories as needed.
pub fn write_files(dir: &Path, files: &[TreeFile]) -> io::Result<()> {
    for (rel, bytes) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, bytes)?;
    }
    Ok(())
}

/// Atomically point `out/current` at `snap`: write a temp symlink then rename it
/// over `current`. rename(2) is atomic, so a reader resolves either the old
/// target or the new one — never a missing/half-built link. The link is relative
/// (`current -> out-<ts>`) so it stays valid under any mount path.
pub fn flip_current(out: &Path, snap: &Path) -> io::Result<()> {
    let target = snap.file_name().expect("snapshot dir has a file name");
    let tmp = out.join(format!(".current.tmp.{}", std::process::id()));
    let _ = fs::remove_file(&tmp);
    symlink(target, &tmp)?;
    fs::rename(&tmp, out.join("current"))
}

/// Remove old `out-*` snapshots, keeping the newest `keep` and never the one
/// `out/current` resolves to (never delete what's being served).
pub fn gc(out: &Path, keep: usize) -> io::Result<()> {
    let mut snaps: Vec<PathBuf> = fs::read_dir(out)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("out-"))
        })
        .collect();
    snaps.sort(); // nanosecond names sort chronologically; newest last
    let n = snaps.len();
    // The snapshot `current` points at, protected even if it's not among the
    // newest `keep`.
    let current = fs::read_link(out.join("current"))
        .ok()
        .and_then(|t| t.file_name().map(|f| f.to_os_string()));
    for (i, p) in snaps.iter().enumerate() {
        let is_recent = i + keep >= n;
        let is_current = p.file_name() == current.as_deref();
        if !is_recent && !is_current {
            let _ = fs::remove_dir_all(p);
        }
    }
    Ok(())
}

/// Convenience: [`new_snapshot`] -> [`write_files`] -> [`flip_current`] ->
/// [`gc`]. Returns the snapshot directory. Use this when you have a prebuilt file
/// map; drive the primitives yourself if you render directly into the snapshot.
pub fn publish(out: &Path, files: &[TreeFile], keep: usize) -> io::Result<PathBuf> {
    let snap = new_snapshot(out)?;
    write_files(&snap, files)?;
    flip_current(out, &snap)?;
    gc(out, keep)?;
    Ok(snap)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp dir for a test, removed on drop.
    struct TmpDir(PathBuf);
    impl TmpDir {
        fn new(tag: &str) -> TmpDir {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p =
                std::env::temp_dir().join(format!("gopher-core-{tag}-{}-{ts}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            TmpDir(p)
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn publish_writes_tree_and_flips_current() {
        let tmp = TmpDir::new("publish");
        let files: Vec<TreeFile> = vec![
            ("index.gph".to_string(), b"root menu\n".to_vec()),
            ("posts/hello.txt".to_string(), b"a post body\n".to_vec()),
        ];

        let snap = publish(&tmp.0, &files, 3).unwrap();

        // current is a symlink to a relative out-* target
        let link = tmp.0.join("current");
        let target = fs::read_link(&link).unwrap();
        assert!(target.to_str().unwrap().starts_with("out-"));
        assert_eq!(tmp.0.join(&target), snap);

        // resolving current/ yields a complete tree, nested dirs and all
        assert_eq!(
            fs::read_to_string(link.join("index.gph")).unwrap(),
            "root menu\n"
        );
        assert_eq!(
            fs::read_to_string(link.join("posts/hello.txt")).unwrap(),
            "a post body\n"
        );
    }

    #[test]
    fn gc_keeps_recent_plus_current_and_drops_the_rest() {
        let tmp = TmpDir::new("gc");
        // Six chronological snapshots: out-000 .. out-005
        let mut dirs = Vec::new();
        for i in 0..6 {
            let d = tmp.0.join(format!("out-{i:03}"));
            fs::create_dir_all(&d).unwrap();
            dirs.push(d);
        }
        // Pretend out-000 is the current target (oldest) — must be retained even
        // though it's not among the newest 3.
        symlink("out-000", tmp.0.join("current")).unwrap();
        gc(&tmp.0, 3).unwrap();

        let remaining: std::collections::BTreeSet<String> = fs::read_dir(&tmp.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with("out-"))
            .collect();
        // newest 3 (003,004,005) + the protected current (000) = 4
        assert_eq!(remaining.len(), 4, "remaining: {remaining:?}");
        assert!(remaining.contains("out-000")); // current protected
        assert!(remaining.contains("out-005")); // newest
        assert!(!remaining.contains("out-001")); // dropped
        assert!(!remaining.contains("out-002")); // dropped
    }

    #[test]
    fn gc_respects_keep_count() {
        let tmp = TmpDir::new("gckeep");
        for i in 0..5 {
            fs::create_dir_all(tmp.0.join(format!("out-{i:03}"))).unwrap();
        }
        symlink("out-004", tmp.0.join("current")).unwrap();
        gc(&tmp.0, 2).unwrap();
        let n = fs::read_dir(&tmp.0)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap().starts_with("out-"))
            .count();
        // newest 2 (003,004); 004 is also current -> 2 total
        assert_eq!(n, 2);
    }
}
