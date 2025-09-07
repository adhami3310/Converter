use std::{env::var, path::PathBuf};
use tempfile::{TempDir, tempdir, tempdir_in};

use crate::magick::JobFile;

pub async fn create_temporary_dir() -> std::io::Result<TempDir> {
    if ashpd::is_sandboxed().await {
        let prefix = format!("{}/tmp", var("XDG_CACHE_HOME").unwrap());
        Ok(tempdir_in(prefix)?)
    } else {
        Ok(tempdir()?)
    }
}

pub fn get_temp_file_path(dir: &TempDir, identifer: JobFile) -> PathBuf {
    let dir_path = dir.path();
    dir_path.join(identifer.as_filename())
}

pub fn clean_dir(temp_dir_path: String) {
    std::fs::remove_dir_all(temp_dir_path).unwrap();
}
