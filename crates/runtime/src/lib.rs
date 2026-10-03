use directories::ProjectDirs;
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use zip::ZipArchive;

pub fn initialize_config_dir() -> io::Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("owo", "cork", "cork") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.to_path_buf())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "failed to resolve & create config directory",
        ))
    }
}

pub fn extract_libraries(
    apk_path: &Path,
    dest_dir: &Path,
    target_abi: Option<&str>,
) -> io::Result<Vec<PathBuf>> {
    let file = File::open(apk_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut extracted_files = Vec::new();

    let prefix = match target_abi {
        Some(abi) => format!("lib/{abi}/"),
        None => "lib/".to_string(),
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        // sanitize path
        let Some(enclosed_path) = entry.enclosed_name().map(|p| p.to_owned()) else {
            continue;
        };

        // only entries that start with desired path
        if !enclosed_path.starts_with(&prefix) {
            continue;
        };

        let out_path = dest_dir.join(&enclosed_path);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&out_path)?;
            io::copy(&mut entry, &mut out_file)?;

            // preserve unix perms
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
                }
            }

            extracted_files.push(out_path);
        }
    }

    Ok(extracted_files)
}
