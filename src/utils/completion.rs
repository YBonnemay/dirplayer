use std::{
    fs,
    path::{Path, PathBuf},
};

/// Returns the names of child directories inside `path`.
///
/// Non-directory entries are ignored. If `path` cannot be read, an empty
/// vector is returned.
///
/// The returned values are directory names only, not full paths.
pub fn get_path_completions(path: &Path) -> Vec<PathBuf> {
    match fs::read_dir(path) {
        Ok(paths) => paths
            .filter_map(|e| {
                let e_ref = e.as_ref().ok()?;
                let metadata = e_ref.metadata().ok()?;
                if !metadata.is_dir() {
                    return None;
                }
                Some(PathBuf::from(e_ref.path()))
            })
            .collect::<Vec<PathBuf>>(),
        Err(_) => vec![],
    }
}
