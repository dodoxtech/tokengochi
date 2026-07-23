use std::path::PathBuf;

#[cfg(debug_assertions)]
pub const APP_DATA_DIR_NAME: &str = "com.tokengochi.dev";
#[cfg(not(debug_assertions))]
pub const APP_DATA_DIR_NAME: &str = "com.tokengochi.app";

#[cfg(debug_assertions)]
pub const WATCHER_DATA_DIR_NAME: &str = "tokengochi-dev";
#[cfg(not(debug_assertions))]
pub const WATCHER_DATA_DIR_NAME: &str = "tokengochi";

pub fn app_data_dir() -> PathBuf {
    data_dir_or_current().join(APP_DATA_DIR_NAME)
}

pub fn watcher_data_dir() -> PathBuf {
    data_dir_or_current().join(WATCHER_DATA_DIR_NAME)
}

pub fn watcher_data_file(file_name: &str) -> PathBuf {
    watcher_data_dir().join(file_name)
}

/// Permanently removes every persisted Tokengochi directory for this build:
/// the SQLite game database (`app_data_dir`) and the watcher bookkeeping files
/// (`watcher_data_dir`). Used by the in-app uninstall action so that either a
/// clean removal or a subsequent fresh install starts from an empty database.
/// A missing directory is treated as already-clean, not an error.
pub fn wipe_all_app_data() -> std::io::Result<()> {
    for dir in [app_data_dir(), watcher_data_dir()] {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn data_dir_or_current() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_data_dir_uses_build_specific_namespace() {
        let path = app_data_dir();
        assert!(path.ends_with(APP_DATA_DIR_NAME));
    }

    #[test]
    fn watcher_data_dir_uses_build_specific_namespace() {
        assert!(watcher_data_dir().ends_with(WATCHER_DATA_DIR_NAME));
    }

    #[test]
    fn watcher_data_file_uses_build_specific_namespace() {
        let path = watcher_data_file("state.json");
        assert!(path.ends_with(PathBuf::from(WATCHER_DATA_DIR_NAME).join("state.json")));
    }
}
