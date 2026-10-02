pub mod cli;

use gtk4::prelude::*;

pub const VERSION: &str = env!("BUILD_VERSION");

pub fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some(&format!("cork {VERSION}")));
    window.set_default_size(350, 70);

    let button = gtk4::Button::with_label("Click me!");
    window.set_child(Some(&button));
    window.present();
}
