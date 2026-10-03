use std::fs;
use std::path::PathBuf;

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
    match runtime::extract_libraries(apk_path, &lib_dir, Some("x86_64")) {
        Ok(files) => {
            println!(
                "extracted {} libraries to {}",
                files.len(),
                lib_dir.display()
            );
        }
        Err(err) => {
            eprintln!("[err] failed to extract libraries: {err}");
            return glib::ExitCode::FAILURE;
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
