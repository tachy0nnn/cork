use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gtk4::prelude::*;
use gtk4::{Application, glib};
use runtime::aurora::download_apk;
use shell::{build_ui, cli};

const APP_ID: &str = "owo.cork.shell";
const ROBLOX_PKG: &str = "com.roblox.client";
const ROBLOX_BASE: &str = "com.roblox.client.apk";
const ABI: &str = "arm64-v8a";

fn main() -> glib::ExitCode {
    let matches = cli::build_cli().get_matches();

    if let Some(("info", _)) = matches.subcommand() {
        println!("version: {}", shell::VERSION);
        println!("aID: {APP_ID}");
        return glib::ExitCode::SUCCESS;
    }

    let debug = matches.get_flag("debug");

    let config_path = match runtime::initialize_config_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("[error] failed to initialize config dir: {err}");
            return glib::ExitCode::FAILURE;
        }
    };
    if debug {
        println!("[dbg] initialized config dir at: {}", config_path.display());
    }

    let apks = match resolve_apks(matches.get_one::<PathBuf>("apk"), &config_path) {
        Ok(apks) => apks,
        Err(err) => {
            eprintln!("[error] {err:#}");
            return glib::ExitCode::FAILURE;
        }
    };

    let lib_dir = config_path.join("lib");
    let assets_dir = config_path.join("assets");
    if let Err(err) = fs::create_dir_all(&lib_dir).and_then(|()| fs::create_dir_all(&assets_dir)) {
        eprintln!("[err] failed to create lib/assets directories: {err}");
        return glib::ExitCode::FAILURE;
    }

    let extracted_libs = match extract_libs(&apks, &lib_dir) {
        Ok(libs) => libs,
        Err(err) => {
            eprintln!("[err] {err:#}");
            return glib::ExitCode::FAILURE;
        }
    };

    if let Err(err) = extract_assets(&apks, &assets_dir) {
        eprintln!("[err] {err:#}");
        return glib::ExitCode::FAILURE;
    }

    let loader = match linker::AndroidLoader::new() {
        Ok(l) => l,
        Err(err) => {
            eprintln!("[err] failed to initialize JNI environment: {err}");
            return glib::ExitCode::FAILURE;
        }
    };

    // shims plus extracted libs, so the linker can find both
    let shims_dir =
        runtime::setup_system_shims(&config_path).unwrap_or_else(|_| config_path.join("shims"));
    let search_path = format!("{}:{}", shims_dir.display(), lib_dir.display());
    loader.set_search_path(Path::new(&search_path));

    if debug {
        println!(
            "[dbg] JNI VM initialized (JavaVM: {:p}, JNIEnv: {:p})",
            loader.vm().java_vm(),
            loader.vm().jni_env()
        );
        println!("[dbg] linker search path set to: {search_path}");
    }

    preload_libs(&loader, &extracted_libs, debug);

    if debug {
        println!("[dbg] launching gtk4 window...");
    }

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}

fn resolve_apks(arg: Option<&PathBuf>, config_dir: &Path) -> Result<Vec<PathBuf>> {
    match arg {
        Some(path) => {
            anyhow::ensure!(path.exists(), "APK not found: {}", path.display());
            println!("using APK: {}", path.display());
            Ok(vec![path.clone()])
        }
        None => ensure_roblox_apks(config_dir),
    }
}

fn ensure_roblox_apks(config_dir: &Path) -> Result<Vec<PathBuf>> {
    let pkg_dir = config_dir.join(ROBLOX_PKG);
    let mut apks = find_apks(&pkg_dir);

    if apks.is_empty() {
        println!("downloading APK from google play");
        download_apk(ROBLOX_PKG.to_string(), config_dir)?;
        apks = find_apks(&pkg_dir);
    }

    anyhow::ensure!(
        !apks.is_empty(),
        "no .apk files found in {}",
        pkg_dir.display()
    );

    apks.sort_by_key(|p| p.file_name() != Some(OsStr::new(ROBLOX_BASE)));
    Ok(apks)
}

fn find_apks(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("apk"))
        .collect()
}

fn extract_libs(apks: &[PathBuf], lib_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut libs = Vec::new();
    for apk in apks {
        let files = runtime::extract_libraries(apk, lib_dir, Some(ABI))
            .with_context(|| format!("failed to extract libraries from {}", apk.display()))?;
        libs.extend(files);
    }
    println!(
        "extracted {} libraries to {}",
        libs.len(),
        lib_dir.display()
    );
    Ok(libs)
}

fn extract_assets(apks: &[PathBuf], assets_dir: &Path) -> Result<()> {
    let mut count = 0;
    for apk in apks {
        let files = runtime::extract_assets_dir(apk, assets_dir)
            .with_context(|| format!("failed to extract assets from {}", apk.display()))?;
        count += files.len();
    }
    println!("extracted {count} asset files to {}", assets_dir.display());
    Ok(())
}

fn preload_libs(loader: &linker::AndroidLoader, libs: &[PathBuf], debug: bool) {
    for lib_path in libs {
        match loader.load_library(lib_path) {
            Ok(version) if version > 0 => {
                println!(
                    "[linker] loaded {} (JNI version: 0x{version:x})",
                    lib_path.display()
                );
            }
            Ok(_) if debug => println!("[dbg] loaded {}", lib_path.display()),
            Err(err) if debug => eprintln!("[linker] note: {err}"),
            Ok(_) | Err(_) => {}
        }
    }
}
