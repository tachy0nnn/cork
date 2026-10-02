use gtk4::prelude::*;
use gtk4::{Application, glib};
use shell::{build_ui, cli};

const APP_ID: &str = "owo.cork.shell";

fn main() -> glib::ExitCode {
    // cli first
    let matches = cli::build_cli().get_matches();
    match matches.subcommand() {
        Some(("info", _)) => {
            println!("version: {}", shell::VERSION);
            println!("aID: {APP_ID}");
            return glib::ExitCode::SUCCESS;
        }
        _ => {}
    }

    if matches.get_flag("debug") {
        println!("[dbg] launching gtk4 window...");
    }

    // then gtk ui
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}