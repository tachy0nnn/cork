use crc32fast::Hasher;
use directories::ProjectDirs;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
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

fn compute_file_crc32(path: &Path) -> io::Result<u32> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 16384];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize())
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
        let Some(enclosed_path) = entry.enclosed_name() else {
            continue;
        };

        // only entries that start with desired path
        if !enclosed_path.starts_with(&prefix) || entry.is_dir() {
            continue;
        }

        // strip the prefix so files go directly under dest_dir
        // e.g. "lib/x86_64/libtest.so" -> "libtest.so"
        let Ok(relative_path) = enclosed_path.strip_prefix(&prefix) else {
            continue;
        };

        let out_path = dest_dir.join(relative_path);

        let is_up_to_date = fs::metadata(&out_path).is_ok_and(|meta| meta.len() == entry.size())
            && compute_file_crc32(&out_path).is_ok_and(|crc| crc == entry.crc32());

        if is_up_to_date {
            extracted_files.push(out_path);
            continue;
        }

        // file is missing or has a different CRC/size
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = BufWriter::new(File::create(&out_path)?);
        io::copy(&mut entry, &mut outfile)?;
        outfile.flush()?;

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

pub fn extract_assets_dir(apk_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let file = File::open(apk_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut extracted_files = Vec::new();

    let prefix = "assets/";

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }

        // sanitize path
        let Some(enclosed_path) = entry.enclosed_name() else {
            continue;
        };

        // filter only assets
        if !enclosed_path.starts_with(prefix) {
            continue;
        }

        // strip "assets/" prefix so files sit relative to dest_dir
        let Ok(relative_path) = enclosed_path.strip_prefix(prefix) else {
            continue;
        };

        let out_path = dest_dir.join(relative_path);

        let is_up_to_date = fs::metadata(&out_path).is_ok_and(|meta| meta.len() == entry.size())
            && compute_file_crc32(&out_path).is_ok_and(|crc| crc == entry.crc32());

        if is_up_to_date {
            extracted_files.push(out_path);
            continue;
        }

        // file is missing or has a different CRC/size
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = BufWriter::new(File::create(&out_path)?);
        io::copy(&mut entry, &mut outfile)?;
        outfile.flush()?;

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

pub fn setup_system_shims(config_dir: &Path) -> io::Result<PathBuf> {
    let shims_dir = config_dir.join("shims");
    fs::create_dir_all(&shims_dir)?;

    let stale_shims = [
        "libc.so",
        "libm.so",
        "libdl.so",
        "libz.so",
        "libEGL.so",
        "libGLESv2.so",
        "ld-linux-x86-64.so.2",
    ];

    for name in stale_shims {
        let p = shims_dir.join(name);
        if p.is_symlink() || p.exists() {
            let _ = fs::remove_file(&p);
        }
    }

    Ok(shims_dir)
}
