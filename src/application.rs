use glib::{clone, ExitCode};
use log::{debug, info};

use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::input_file::InputFile;
use crate::window::AppWindow;

mod imp {

    use super::*;
    use adw::subclass::prelude::AdwApplicationImpl;

    #[derive(Debug)]
    pub struct App {
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for App {
        const NAME: &'static str = "ConverterApp";
        type Type = super::App;
        type ParentType = adw::Application;

        fn new() -> Self {
            Self {
                settings: gio::Settings::new(APP_ID),
            }
        }
    }

    impl ObjectImpl for App {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.setup_gactions();
            obj.setup_accels();
            obj.setup_settings();
        }
    }

    impl ApplicationImpl for App {
        fn activate(&self) {
            debug!("Application::activate");

            self.obj().present_main_window();
        }

        fn startup(&self) {
            debug!("Application::startup");
            self.parent_startup();

            // Set icons for shell
            gtk::Window::set_default_icon_name(APP_ID);
        }

        fn open(&self, files: &[gio::File], _hint: &str) {
            debug!("Application::open");

            let files: Vec<Option<InputFile>> = files
                .iter()
                .cloned()
                .map(|file| InputFile::new(&file))
                .collect();

            let application = self.obj();
            application.present_main_window();
            if let Some(window) = application.active_window() {
                window
                    .downcast_ref::<AppWindow>()
                    .unwrap()
                    .open_files(files);
            }
        }
    }

    impl gtk::subclass::prelude::GtkApplicationImpl for App {}
    impl AdwApplicationImpl for App {}
}

glib::wrapper! {
    pub struct App(ObjectSubclass<imp::App>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for App {
    fn default() -> Self {
        glib::Object::builder::<Self>()
            .property("application-id", Some(APP_ID))
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .property("resource-base-path", "/io/gitlab/adhami3310/Converter/")
            .build()
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    fn setup_settings(&self) {
        self.imp().settings.connect_changed(
            Some("show-less-popular"),
            clone!(@weak self as this => move |_, _| {
                if let Some(window) = this.active_window() {
                    window.downcast_ref::<AppWindow>().unwrap().update_output_options();
                }
            }),
        );
    }

    fn setup_gactions(&self) {
        self.add_action_entries([
            gio::ActionEntry::builder("quit")
                .activate(clone!(@weak self as app => move |_,_, _| {
                    app.quit();
                }))
                .build(),
            gio::ActionEntry::builder("new-window")
                .activate(clone!(@weak self as app => move |_, _, _| {
                    app.present_main_window();
                }))
                .build(),
        ]);

        let show_hidden = self.imp().settings.boolean("show-less-popular");
        self.add_action_entries([gio::ActionEntry::builder("popular")
            .state(show_hidden.to_variant())
            .activate(clone!(@weak self as this => move |_, action, _| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let show_hidden = !action_state;
                action.set_state(show_hidden.to_variant());

                this.imp()
                    .settings
                    .set_boolean("show-less-popular", show_hidden)
                    .expect("Unable to store show-less-popular setting");
            }))
            .build()]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.new-window", &["<Control>n"]);
        self.set_accels_for_action("app.popular", &["<Control>h"]);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("win.clear", &["<Control>r"]);
        self.set_accels_for_action("win.add", &["<Control>o"]);
        self.set_accels_for_action("win.close", &["<Control>w"]);
        self.set_accels_for_action("win.paste", &["<Control>v"]);
    }

    fn present_main_window(&self) {
        let window = AppWindow::new(self);
        let window: gtk::Window = window.upcast();
        window.present();
    }

    pub fn run(&self) -> ExitCode {
        info!("Converter ({})", APP_ID);
        info!("Version: {} ({})", VERSION, PROFILE);
        info!("Datadir: {}", PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}
