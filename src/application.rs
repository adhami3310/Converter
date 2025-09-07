use glib::{clone, ExitCode};
use log::{debug, error, info};

use gettextrs::gettext;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::input_file::InputFile;
use crate::window::AppWindow;

mod imp {

    use crate::window::FileOperations;

    use super::*;
    use adw::subclass::prelude::AdwApplicationImpl;

    #[derive(Debug)]
    pub struct App {
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for App {
        const NAME: &'static str = "SwitcherooApp";
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

            obj.add_main_option(
                "new-window",
                glib::Char::from(b'w'),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                &gettext("Open a new window"),
                None,
            );

            obj.setup_gactions();
            obj.setup_accels();
        }
    }

    impl ApplicationImpl for App {
        fn activate(&self) {
            debug!("Application::activate");
            self.parent_activate();

            let application = self.obj();
            if let Some(window) = application.active_window() {
                window.present();
            } else {
                application.present_main_window();
            }
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
                .filter(|file| file.query_exists(gio::Cancellable::NONE))
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

        fn handle_local_options(
            &self,
            options: &glib::VariantDict,
        ) -> std::ops::ControlFlow<glib::ExitCode> {
            debug!("Application::handle_local_options");

            let application = self.obj();
            if options.contains("new-window") {
                if let Err(err) = application.register(None::<&gio::Cancellable>) {
                    error!("Failed to register the application: {err}");
                }

                if application.is_remote() {
                    application.activate_action("new-window", None);
                    return std::ops::ControlFlow::Break(glib::ExitCode::SUCCESS);
                }
            }

            self.parent_handle_local_options(options)
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

    fn setup_gactions(&self) {
        self.add_action_entries([
            gio::ActionEntry::builder("quit")
                .activate(clone!(
                    #[weak(rename_to=app)]
                    self,
                    move |_, _, _| {
                        app.quit();
                    }
                ))
                .build(),
            gio::ActionEntry::builder("new-window")
                .activate(clone!(
                    #[weak(rename_to=app)]
                    self,
                    move |_, _, _| {
                        app.present_main_window();
                    }
                ))
                .build(),
        ]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.new-window", &["<Control>n"]);
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
        info!("Switcheroo ({})", APP_ID);
        info!("Version: {} ({})", VERSION, PROFILE);
        info!("Datadir: {}", PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}
