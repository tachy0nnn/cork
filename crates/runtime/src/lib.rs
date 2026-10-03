use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
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

fn compute_file_sha256(path: &Path) -> io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().into())
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
        if !enclosed_path.starts_with(&prefix) || entry.is_dir() {
            continue;
        };

        // strip the prefix so files go directly under dest_dir
        // e.g. "lib/x86_64/libtest.so" -> "libtest.so"
        let relative_path = match enclosed_path.strip_prefix(&prefix) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let out_path = dest_dir.join(relative_path);
        let mut content = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut content)?;

        // hash the APK file
        let apk_sha256: [u8; 32] = Sha256::digest(&content).into();

        // if file exists on disk, compare SHA-256 hashes
        if out_path.exists() {
            if let Ok(local_sha256) = compute_file_sha256(&out_path) {
                if local_sha256 == apk_sha256 {
                    // skip if hashes same
                    continue;
                }
            }
        }

        // file is missing or has a different hash?
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&out_path, &content)?;

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

    Ok(extracted_files)
}
