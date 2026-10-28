use crate::app::App;
use std::{io::Write, path::Path};
pub(crate) fn dialog(app: &mut App) {
    let bytes = match bytes(app) {
        Ok(b) => b,
        Err(error) => {
            app.load_error = Some(error);
            return;
        }
    };
    let name = app
        .saved
        .path()
        .and_then(|p| p.file_stem())
        .unwrap_or_else(|| std::ffi::OsStr::new("song"))
        .to_string_lossy();
    let Some(path) = rfd::FileDialog::new()
        .add_filter("SMF", &["mid", "midi", "smf"])
        .set_file_name(format!("{name}-performance.mid"))
        .save_file()
    else {
        return;
    };
    if let Err(error) = write(app, &path, &bytes) {
        app.load_error = Some(error);
    }
}
fn bytes(app: &App) -> Result<Vec<u8>, String> {
    let m = &app.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    cat_smf_editor::smf_io::encode(&cat_smf_editor::score::performance::song(m)?)
}
fn write(app: &App, path: &Path, bytes: &[u8]) -> Result<(), String> {
    let canonical = |p: &Path| dunce::canonicalize(p).unwrap_or_else(|_| p.to_owned());
    if app
        .saved
        .path()
        .is_some_and(|source| canonical(source) == canonical(path))
    {
        return Err("Performance export must use a different file from the editable MID".into());
    }
    if path.exists() {
        let expected = std::fs::read(path).map_err(|e| e.to_string())?;
        let backup = app
            .saved
            .directory
            .as_ref()
            .ok_or("Backup directory is unavailable")?
            .join("backup");
        crate::store::replace_with_backup(path, bytes, &expected, &backup)
            .map_err(|e| e.to_string())
    } else {
        let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist_noclobber(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_preserves_editable_document_and_refuses_replacing_it() {
        let (mut a, _, _, _, _) = crate::view::patterns::workspace_tests::verification_song();
        let work = tempfile::tempdir().unwrap();
        let source = work.path().join("editable.mid");
        a.saved.directory = Some(work.path().into());
        a.saved.save_as(&source).unwrap();
        let before = std::fs::read(&source).unwrap();
        let dirty = a.saved.dirty();
        let bytes = bytes(&a).unwrap();
        assert!(write(&a, &source, &bytes).is_err());
        let output = work.path().join("performance.mid");
        write(&a, &output, &bytes).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert_eq!(a.saved.dirty(), dirty);
        assert_eq!(a.saved.path(), Some(source.as_path()));
        let music = cat_smf_editor::smf_io::load(&output).unwrap();
        assert!(cat_smf_editor::score::pattern::Catalog::read(&music)
            .unwrap()
            .sources
            .is_empty());
    }
}
