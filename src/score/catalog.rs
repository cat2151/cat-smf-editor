use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub relative: String,
}

pub fn discover(root: &Path) -> Result<Vec<Entry>, String> {
    if !root.is_dir() {
        return Err(format!("SMF directory is missing: {}", root.display()));
    }
    fn visit(root: &Path, dir: &Path, output: &mut Vec<Entry>) -> Result<(), String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Cannot scan {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Cannot scan {}: {e}", dir.display()))?;
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                visit(root, &entry.path(), output)?;
            } else if kind.is_file()
                && entry.path().extension().is_some_and(|ext| {
                    ["mid", "midi", "smf"]
                        .iter()
                        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
                })
            {
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                output.push(Entry { path, relative });
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(root, root, &mut output)?;
    output.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(output)
}
