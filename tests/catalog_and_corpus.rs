use cat_smf_editor::{score::catalog, smf_io};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("cat-smf-catalog-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_empty_duplicate_names_extensions_and_directory_links() {
    let temp = Temporary::new();
    let root = temp.0.join("materials");
    let error = catalog::discover(&root).unwrap_err();
    assert!(error.contains(&root.display().to_string()));
    std::fs::create_dir(&root).unwrap();
    assert!(catalog::discover(&root).unwrap().is_empty());
    for directory in ["a", "b"] {
        std::fs::create_dir(root.join(directory)).unwrap();
    }
    for name in [
        "a/same.MID",
        "b/same.mid",
        "b/other.MiDi",
        "one.SMF",
        "ignored.txt",
    ] {
        std::fs::write(root.join(name), []).unwrap();
    }
    let outside = temp.0.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("external.mid"), []).unwrap();
    let link = root.join("link");
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let result = catalog::discover(&root).unwrap();
    assert_eq!(
        result
            .iter()
            .map(|e| e.relative.as_str())
            .collect::<Vec<_>>(),
        ["a/same.MID", "b/other.MiDi", "b/same.mid", "one.SMF"]
    );
    #[cfg(windows)]
    std::fs::remove_dir(&link).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(&link).unwrap();
}

// Independent test-only SMF decoder: compare every source event's delta,
// status, Meta type and payload with the actual umppi/FFI result.
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Reader<'_> {
    fn byte(&mut self) -> u8 {
        let b = self.bytes[self.offset];
        self.offset += 1;
        b
    }
    fn word(&mut self, n: usize) -> usize {
        (0..n).fold(0, |v, _| v * 256 + usize::from(self.byte()))
    }
    fn vlq(&mut self) -> u32 {
        let mut value = 0;
        loop {
            let b = self.byte();
            value = value * 128 + u32::from(b & 127);
            if b < 128 {
                return value;
            }
        }
    }
    fn take(&mut self, n: usize) -> &[u8] {
        let start = self.offset;
        self.offset += n;
        &self.bytes[start..self.offset]
    }
}
fn compare_source(path: &Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    let loaded = smf_io::load(path).unwrap_or_else(|e| panic!("{e}"));
    let mut r = Reader {
        bytes: &bytes,
        offset: 0,
    };
    assert_eq!(r.take(4), b"MThd");
    assert_eq!(r.word(4), 6);
    assert_eq!(r.word(2), usize::from(loaded.format));
    assert_eq!(r.word(2), loaded.tracks.len());
    assert_eq!(r.word(2), usize::from(loaded.division));
    let mut total = 0;
    for track in &loaded.tracks {
        assert_eq!(r.take(4), b"MTrk");
        let length = r.word(4);
        let end = r.offset + length;
        let mut running = 0;
        let mut tick = 0u64;
        let mut count = 0;
        while r.offset < end {
            let delta = r.vlq();
            tick += u64::from(delta);
            let status = if r.bytes[r.offset] >= 128 {
                running = r.byte();
                running
            } else {
                running
            };
            let (meta, payload) = if status == 0xff || status == 0xf0 || status == 0xf7 {
                let meta = if status == 0xff { r.byte() } else { 0 };
                let length = r.vlq() as usize;
                (meta, r.take(length).to_vec())
            } else {
                let length = if status & 0xf0 == 0xc0 || status & 0xf0 == 0xd0 {
                    1
                } else {
                    2
                };
                (0, r.take(length).to_vec())
            };
            let event = &track.events[count];
            assert_eq!(
                (
                    event.delta,
                    event.tick,
                    event.status,
                    event.meta_type,
                    &event.data
                ),
                (delta, tick, status, meta, &payload),
                "{} event {}",
                path.display(),
                count
            );
            count += 1;
        }
        assert_eq!(r.offset, end);
        assert_eq!(count, track.events.len());
        total += count;
    }
    assert_eq!(r.offset, bytes.len());
    total
}

#[test]
fn all_materials_match_disk_and_preserve_source_events() {
    fn disk_files(directory: &Path, output: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_symlink() {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                disk_files(&entry.path(), output);
            } else if let Some(ext) = entry.path().extension()
                && matches!(
                    ext.to_str().unwrap().to_ascii_uppercase().as_str(),
                    "MID" | "MIDI" | "SMF"
                )
            {
                output.insert(entry.path());
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cat-music-patterns");
    let files = catalog::discover(&root).unwrap();
    let mut disk = BTreeSet::new();
    disk_files(&root, &mut disk);
    assert!(!disk.is_empty(), "required material corpus is empty");
    assert_eq!(
        files
            .iter()
            .map(|e| e.path.clone())
            .collect::<BTreeSet<_>>(),
        disk
    );
    let total: usize = files.iter().map(|entry| compare_source(&entry.path)).sum();
    println!("{} SMFs, {total} original events verified", files.len());
}
