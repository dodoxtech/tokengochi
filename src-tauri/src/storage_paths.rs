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

pub fn watcher_data_file(file_name: &str) -> PathBuf {
    data_dir_or_current()
        .join(WATCHER_DATA_DIR_NAME)
        .join(file_name)
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
    fn watcher_data_file_uses_build_specific_namespace() {
        let path = watcher_data_file("state.json");
        assert!(path.ends_with(PathBuf::from(WATCHER_DATA_DIR_NAME).join("state.json")));
    }
}
