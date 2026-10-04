use std::fs;
use std::path::{Path, PathBuf};

use gtk4::prelude::*;
use gtk4::{Application, glib};
use shell::{build_ui, cli};

const APP_ID: &str = "owo.cork.shell";

fn main() -> glib::ExitCode {
    // cli first
    let matches = cli::build_cli().get_matches();

    // SHUT UP I AM SO ANNOYED RIGHT NOW
    #[allow(clippy::single_match)]
    match matches.subcommand() {
        Some(("info", _)) => {
            println!("version: {}", shell::VERSION);
            println!("aID: {APP_ID}");
            return glib::ExitCode::SUCCESS;
        }
        _ => {}
    }

    let apk_path: &PathBuf = matches
        .get_one::<PathBuf>("apk")
        .expect("APK path is required");

    println!("using APK: {}", apk_path.display());

    // initialize config dir
    let config_path = match runtime::initialize_config_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("[error] failed to initialize config dir: {err}");
            return glib::ExitCode::FAILURE;
        }
    };

    if matches.get_flag("debug") {
        println!("[dbg] initialized config dir at: {}", config_path.display());
    }

    // 'lib' folder inside config directory
    let lib_dir = config_path.join("lib");
    if let Err(err) = fs::create_dir_all(&lib_dir) {
        eprintln!("[err] failed to create lib directory: {err}");
        return glib::ExitCode::FAILURE;
    }

    // extract libs
    let extracted_libs = match runtime::extract_libraries(apk_path, &lib_dir, Some("x86_64")) {
        Ok(files) => {
            println!(
                "extracted {} libraries to {}",
                files.len(),
                lib_dir.display()
            );
            files
        }
        Err(err) => {
            eprintln!("[err] failed to extract libraries: {err}");
            return glib::ExitCode::FAILURE;
        }
    };

    // 'assets' folder inside cfg dir
    let assets_dir = config_path.join("assets");
    if let Err(err) = fs::create_dir_all(&assets_dir) {
        eprintln!("[err] failed to create assets directory: {err}");
        return glib::ExitCode::FAILURE;
    }

    // extract `assets/` dir
    match runtime::extract_assets_dir(apk_path, &assets_dir) {
        Ok(files) => {
            println!(
                "extracted {} asset files to {}",
                files.len(),
                assets_dir.display()
            );
        }
        Err(err) => {
            eprintln!("[err] failed to extract assets: {err}");
            return glib::ExitCode::FAILURE;
        }
    }

    // init JVM and loader
    let loader = match linker::AndroidLoader::new() {
        Ok(l) => l,
        Err(err) => {
            eprintln!("[err] failed to initialize JNI environment: {err}");
            return glib::ExitCode::FAILURE;
        }
    };

    // setup automatically shims and configure search path so the linker finds both shims and extracted libs
    let shims_dir =
        runtime::setup_system_shims(&config_path).unwrap_or_else(|_| config_path.join("shims"));
    let search_path = format!("{}:{}", shims_dir.display(), lib_dir.display());
    loader.set_search_path(Path::new(&search_path));

    if matches.get_flag("debug") {
        println!(
            "[dbg] JNI VM initialized (JavaVM: {:p}, JNIEnv: {:p})",
            loader.vm().java_vm(),
            loader.vm().jni_env()
        );
        println!("[dbg] linker search path set to: {search_path}");
    }

    // debugging
    for lib_path in &extracted_libs {
        match loader.load_library(lib_path) {
            Ok(version) if version > 0 => {
                println!(
                    "[linker] loaded {} (JNI version: 0x{version:x})",
                    lib_path.display()
                );
            }
            Ok(_) => {
                if matches.get_flag("debug") {
                    println!("[dbg] loaded {}", lib_path.display());
                }
            }
            Err(err) => {
                if matches.get_flag("debug") {
                    eprintln!("[linker] note: {err}");
                }
            }
        }
    }

    if matches.get_flag("debug") {
        println!("[dbg] launching gtk4 window...");
    }

    // then gtk ui
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}
