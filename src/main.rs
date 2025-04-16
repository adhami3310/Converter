mod application;
#[rustfmt::skip]
mod config;
mod color;
mod drag_overlay;
mod file_chooser;
mod filetypes;
mod input_file;
mod magick;
mod temp;
mod widgets;
mod window;

use std::sync::OnceLock;

use gettextrs::{gettext, LocaleCategory};
use glib::ExitCode;
use gtk::{gio, glib};
use tokio::runtime::Runtime;

use self::application::App;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

const GHOST_SCRIPT_BINARY_NAME: &str = "gs";
const ZIP_BINARY_NAME: &str = "zip";

fn main() -> ExitCode {
    // Initialize logger
    pretty_env_logger::init();

    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Switcheroo"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = App::new();
    app.run()
}
