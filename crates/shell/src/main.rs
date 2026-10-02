use gtk4::prelude::*;
use gtk4::{glib, Application};
use shell::build_ui;

const APP_ID: &str = "owo.cork.shell";

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run()
}