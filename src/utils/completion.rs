use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

/// Returns the names of child directories inside `path`.
///
/// Non-directory entries are ignored. If `path` cannot be read, an empty
/// vector is returned.
///
/// The returned values are directory names only, not full paths.
pub fn path_to_completions(path: &Path) -> VecDeque<PathBuf> {
    match fs::read_dir(path) {
        Ok(paths) => paths
            .filter_map(|e| {
                let e_ref = e.as_ref().ok()?;
                let metadata = e_ref.metadata().ok()?;
                if !metadata.is_dir() {
                    return None;
                }
                e_ref.path().strip_prefix(&path).ok().map(PathBuf::from)
            })
            .collect(),
        Err(_) => VecDeque::default(),
    }
}
