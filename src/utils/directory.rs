use itertools::Itertools;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use walkdir::{DirEntry, WalkDir};

use super::config::{ExtensionArchive, ExtensionFile};

/// The configured extension matching `path`, if any.
pub fn extension_file(path: &Path) -> Option<ExtensionFile> {
    let extension = path.extension()?.to_string_lossy().into_owned();
    ExtensionFile::iter().find(|candidate| candidate.matches_str(&extension))
}

/// Whether the path carries an extension an audio backend can decode.
pub fn is_playable(path: &Path) -> bool {
    extension_file(path).is_some_and(|extension| extension.is_playable())
}

pub fn get_files_archive(path: &Path) -> Vec<DirEntry> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(move |e| {
            let dir_entry = e.as_ref().ok()?;
            let file_extension = dir_entry.path().extension()?.to_string_lossy().to_string();
            let matches_extension_archive =
                ExtensionArchive::iter().any(|extension| extension.matches_str(&file_extension));
            if dir_entry.file_type().is_file() && matches_extension_archive {
                e.ok()
            } else {
                None
            }
        })
        .collect_vec()
}

/// Uncompress archives to cache directory
pub fn process_archives(path: &String, cache_dir: &PathBuf) {
    let files = get_files_archive(&PathBuf::from(path));

    files
        .iter()
        .filter_map(|archive| {
            let file_name = PathBuf::from(archive.file_name());
            let archive_name = file_name.file_stem()?;
            let mut archive_directory = cache_dir.clone();
            archive_directory.push(archive_name);
            if archive_directory.as_path().exists() {
                return None;
            }

            let archive_file = fs::File::open(archive.path()).ok()?;
            let mut archive = zip::ZipArchive::new(archive_file).ok()?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i).ok()?;
                let outpath_name = file.enclosed_name()?.to_owned();
                let mut outpath_dir = archive_directory.clone();
                log::debug!("Building archive : {:?}", outpath_dir);
                outpath_dir.push(&outpath_name);
                if file.is_dir() {
                    fs::create_dir_all(&outpath_dir).unwrap();
                } else {
                    if let Some(p) = outpath_dir.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p).unwrap();
                        }
                    }
                    let mut outfile = fs::File::create(&outpath_dir).unwrap();
                    io::copy(&mut file, &mut outfile).unwrap();
                }
            }
            Some(())
        })
        .collect_vec();
}
