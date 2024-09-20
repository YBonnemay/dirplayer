use itertools::Itertools;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

pub fn get_direntries(path: &Path, extentions: &[String]) -> Vec<DirEntry> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(move |e| {
            let dir_entry = e.as_ref().ok()?;
            let extension_archives = dir_entry.path().extension()?.to_string_lossy().to_string();
            if dir_entry.file_type().is_file() && extentions.contains(&extension_archives) {
                e.ok()
            } else {
                None
            }
        })
        .collect_vec()
}

/// Uncompress archives to cache directory
pub fn process_archives(paths: &[String], extensions_archives: &Vec<String>, cache_dir: &PathBuf) {
    let archive_lines = paths
        .iter()
        // Avoid entirely scanning cache directory
        .filter(|direntry| **direntry != cache_dir.to_string_lossy())
        .flat_map(|direntry| get_direntries(&PathBuf::from(direntry), extensions_archives))
        .collect_vec();

    archive_lines
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
