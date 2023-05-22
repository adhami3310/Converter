use std::path::Path;
use std::sync::atomic::AtomicUsize;

use crate::color::Color;
use crate::config::APP_ID;
use crate::drag_overlay::DragOverlay;
use crate::file_chooser::FileChooser;
use crate::filetypes::{CompressionType, FileType, OutputType};
use crate::input_file::InputFile;
use crate::magick::{
    count_frames, wait_for_child, ConvertJob, GhostScriptConvertJob, JobFile, MagickConvertJob,
    ResizeArgument, SizeArgument,
};
use crate::temp::{clean_dir, create_temporary_dir, get_temp_file_path};
use adw::{prelude::*, traits::ActionRowExt};
use futures::future::join_all;
use gettextrs::gettext;
use glib::clone;
use gtk::ColorDialog;
use gtk::{gdk, gdk::gdk_pixbuf::Pixbuf, gio, glib, subclass::prelude::*};
use itertools::Itertools;
use shared_child::SharedChild;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::spawn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFilter {
    Point,
    Quadratic,
    Cubic,
    Mitchell,
    Gaussian,
    Lanczos,
}

enum ArcOrOptionError {
    Child(Arc<SharedChild>),
    OptionError(Option<String>),
}

#[allow(dead_code)]
impl ResizeFilter {
    pub fn as_display_string(&self) -> &str {
        match self {
            ResizeFilter::Point => "Point",
            ResizeFilter::Quadratic => "Quadratic",
            ResizeFilter::Cubic => "Cubic",
            ResizeFilter::Mitchell => "Mitchell",
            ResizeFilter::Gaussian => "Gaussian",
            ResizeFilter::Lanczos => "Lanczos",
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ResizeFilter::Point),
            1 => Some(ResizeFilter::Quadratic),
            2 => Some(ResizeFilter::Cubic),
            3 => Some(ResizeFilter::Mitchell),
            4 => Some(ResizeFilter::Gaussian),
            5 => Some(ResizeFilter::Lanczos),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ResizeType {
    Percentage,
    ExactPixels,
    MinPixels,
    MaxPixels,
    Ratio,
}

impl ResizeType {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ResizeType::Percentage),
            1 => Some(ResizeType::ExactPixels),
            2 => Some(ResizeType::MinPixels),
            3 => Some(ResizeType::MaxPixels),
            4 => Some(ResizeType::Ratio),
            _ => None,
        }
    }
}

mod imp {
    use std::{cell::RefCell, sync::atomic::AtomicBool};

    use super::*;

    use adw::subclass::prelude::AdwApplicationWindowImpl;
    use gtk::CompositeTemplate;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/gitlab/adhami3310/Converter/blueprints/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub drag_overlay: TemplateChild<DragOverlay>,
        #[template_child]
        pub back: TemplateChild<gtk::Button>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub open_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub convert_button: TemplateChild<gtk::Button>,
        // #[template_child]
        // pub options_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub loading_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub image_container: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub supported_output_filetypes: TemplateChild<gtk::StringList>,
        #[template_child]
        pub supported_compression_filetypes: TemplateChild<gtk::StringList>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        pub image_type_label: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub image_size_label: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub output_filetype: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub output_compression: TemplateChild<adw::ComboRow>,

        #[template_child]
        pub quality: TemplateChild<gtk::Scale>,
        #[template_child]
        pub bgcolor: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub resize_filters: TemplateChild<gtk::StringList>,
        #[template_child]
        pub resize_filter: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub resize_type: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub resize_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_minmax_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_minmax_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_scale_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_scale_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub svg_size_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub svg_size_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub svg_size_type: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub ratio_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub ratio_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub dpi_value: TemplateChild<gtk::Entry>,

        #[template_child]
        pub quality_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub bgcolor_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub svg_size_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub svg_size_width_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub svg_size_height_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_width_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub resize_height_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub resize_minmax_width_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_minmax_height_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_scale_width_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_scale_height_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub ratio_width_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub ratio_height_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub dpi_row: TemplateChild<adw::ActionRow>,

        pub provider: gtk::CssProvider,
        pub input_file_store: gio::ListStore,
        pub settings: gio::Settings,
        pub is_canceled: std::sync::Arc<AtomicBool>,
        pub current_jobs: RefCell<Vec<Arc<SharedChild>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppWindow {
        const NAME: &'static str = "AppWindow";
        type Type = super::AppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                toast_overlay: TemplateChild::default(),
                drag_overlay: TemplateChild::default(),
                back: TemplateChild::default(),
                stack: TemplateChild::default(),
                open_button: TemplateChild::default(),
                convert_button: TemplateChild::default(),
                // options_button: TemplateChild::default(),
                cancel_button: TemplateChild::default(),
                loading_spinner: TemplateChild::default(),
                image: TemplateChild::default(),
                image_container: TemplateChild::default(),
                supported_output_filetypes: TemplateChild::default(),
                supported_compression_filetypes: TemplateChild::default(),
                progress_bar: TemplateChild::default(),
                image_type_label: TemplateChild::default(),
                image_size_label: TemplateChild::default(),
                output_filetype: TemplateChild::default(),
                output_compression: TemplateChild::default(),
                quality: TemplateChild::default(),
                bgcolor: TemplateChild::default(),
                resize_filters: TemplateChild::default(),
                resize_filter: TemplateChild::default(),
                resize_type: TemplateChild::default(),
                resize_width_value: TemplateChild::default(),
                resize_height_value: TemplateChild::default(),
                resize_minmax_width_value: TemplateChild::default(),
                resize_minmax_height_value: TemplateChild::default(),
                resize_scale_width_value: TemplateChild::default(),
                resize_scale_height_value: TemplateChild::default(),
                svg_size_height_value: TemplateChild::default(),
                svg_size_width_value: TemplateChild::default(),
                svg_size_type: TemplateChild::default(),
                ratio_height_value: TemplateChild::default(),
                ratio_width_value: TemplateChild::default(),
                dpi_value: TemplateChild::default(),
                quality_row: TemplateChild::default(),
                bgcolor_row: TemplateChild::default(),
                resize_row: TemplateChild::default(),
                svg_size_row: TemplateChild::default(),
                svg_size_width_row: TemplateChild::default(),
                svg_size_height_row: TemplateChild::default(),
                resize_width_row: TemplateChild::default(),
                resize_height_row: TemplateChild::default(),
                resize_minmax_width_row: TemplateChild::default(),
                resize_minmax_height_row: TemplateChild::default(),
                resize_scale_width_row: TemplateChild::default(),
                resize_scale_height_row: TemplateChild::default(),
                ratio_width_row: TemplateChild::default(),
                ratio_height_row: TemplateChild::default(),
                dpi_row: TemplateChild::default(),
                provider: gtk::CssProvider::new(),
                input_file_store: gio::ListStore::new(InputFile::static_type()),

                settings: gio::Settings::new(APP_ID),
                is_canceled: std::sync::Arc::new(AtomicBool::new(false)),
                current_jobs: RefCell::new(Vec::new()),
            }
        }
    }

    impl ObjectImpl for AppWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.load_window_size();
        }
    }

    impl WidgetImpl for AppWindow {}
    impl WindowImpl for AppWindow {
        fn close_request(&self) -> gtk::Inhibit {
            if let Err(err) = self.obj().save_window_size() {
                dbg!("Failed to save window state, {}", &err);
            }

            // Pass close request on to the parent
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}
}

glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindow>)
        @extends gtk::Widget, gtk::Window,  gtk::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

#[gtk::template_callbacks]
impl AppWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P) -> Self {
        let win = glib::Object::builder::<AppWindow>()
            .property("application", app)
            .build();

        win.setup_callbacks();
        win.setup_provider();
        win.setup_drop_target();

        win
    }

    /// Shows a basic toast with the given text.
    fn show_toast(&self, text: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(text));
    }

    fn setup_callbacks(&self) {
        //load imp
        let imp = self.imp();
        imp.open_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.open_dialog();
            }));
        imp.convert_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.save_files();
            }));
        imp.cancel_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.convert_cancel();
            }));
        // imp.options_button
        //     .connect_clicked(clone!(@weak self as this => move |_| {
        //         this.imp().back.set_visible(true);
        //         this.imp().stack.set_visible_child_name("options_page");
        //     }));
        imp.back
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.imp().back.set_visible(false);
                this.imp().stack.set_visible_child_name("stack_convert");
            }));
        imp.output_filetype
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_advanced_options();
                this.update_compression_options();
            }));
        imp.resize_row
            .connect_expanded_notify(clone!(@weak self as this => move |_| {
                this.update_resize();
            }));
        imp.resize_type
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_resize();
            }));
        imp.resize_width_row
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_resize();
            }));
        imp.resize_height_row
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_resize();
            }));
        imp.svg_size_row
            .connect_expanded_notify(clone!(@weak self as this => move |_| {
                this.update_svg_size();
            }));
        imp.svg_size_type
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_svg_size();
            }));
    }

    fn setup_provider(&self) {
        // let imp = self.imp();
        // if let Some(display) = gtk::gdk::Display::default() {
        //     gtk::StyleContext::add_provider_for_display(&display, &imp.provider, 400);
        // }
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn save_options(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        imp.settings
            .set_int("quality", imp.quality.value() as i32)?;
        imp.settings
            .set_int("dpi", imp.dpi_value.text().parse().unwrap())?;

        Ok(())
    }

    fn load_options(&self) {
        let imp = self.imp();

        imp.quality.set_value(imp.settings.int("quality") as f64);
        imp.dpi_value.set_text(&imp.settings.int("dpi").to_string());
    }

    fn save_selected_output(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let output_format = self.get_selected_output().unwrap();

        let pos = FileType::output_formats(true)
            .position(|&x| x == output_format)
            .unwrap();

        imp.settings.set_enum("output-format", pos as i32)?;

        Ok(())
    }

    fn load_selected_output(&self) -> FileType {
        let imp = self.imp();

        *FileType::output_formats(true).collect_vec()[imp.settings.enum_("output-format") as usize]
    }

    fn save_selected_compression(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        if let Some(output_format) = self.get_selected_compression() {
            let pos = CompressionType::possible_output(false)
                .position(|&x| x == output_format)
                .unwrap();

            imp.settings.set_enum("compression-format", pos as i32)?;
        }
        Ok(())
    }

    fn load_selected_compression(&self) -> CompressionType {
        let imp = self.imp();

        *CompressionType::possible_output(false).collect_vec()
            [imp.settings.enum_("compression-format") as usize]
    }

    fn set_convert_progress(&self, done: usize, total: usize) {
        let msg = format!("{done}/{total}");
        self.imp().progress_bar.set_text(Some(&msg));
        self.imp()
            .progress_bar
            .set_fraction((done as f64) / (total as f64));
    }

    fn set_collecting_progress(&self) {
        let msg = gettext("Collecting files");
        self.imp().progress_bar.set_text(Some(&msg));
    }

    fn setup_drop_target(&self) {
        let drop_target = gtk::DropTarget::builder()
            .name("file-drop-target")
            .actions(gdk::DragAction::COPY)
            .formats(&gdk::ContentFormats::for_type(gdk::FileList::static_type()))
            .build();

        drop_target.connect_drop(
            clone!(@weak self as win => @default-return false, move |_, value, _, _| {
                if let Ok(file_list) = value.get::<gdk::FileList>() {
                    if file_list.files().is_empty() {
                        win.show_toast(&gettext("Unable to access dropped files"));
                        return false;
                    }

                    let mut input_files: Vec<Option<InputFile>> = Vec::new();
                    for f in file_list.files() {
                        input_files.push(InputFile::new(&f));
                    }
                    win.open_files(input_files);
                    return true;
                }

                false
            }),
        );

        self.imp().drag_overlay.set_drop_target(&drop_target);
    }

    pub fn open_dialog(&self) {
        FileChooser::open_files_wrapper(
            self,
            vec![],
            AppWindow::open_load,
            AppWindow::open_success_wrapper,
            AppWindow::open_error,
        );
    }

    fn open_error(&self, error: Option<&str>) {
        match error {
            Some(_) => self
                .imp()
                .stack
                .set_visible_child_name("stack_invalid_image"),
            None if self.imp().input_file_store.n_items() > 0 => {
                self.imp().stack.set_visible_child_name("stack_convert")
            }
            None => self
                .imp()
                .stack
                .set_visible_child_name("stack_welcome_page"),
        };
    }

    fn open_load(&self) {
        self.imp().back.set_visible(false);
        self.imp().stack.set_visible_child_name("stack_loading");
        self.imp().loading_spinner.start();
    }

    fn open_success_wrapper(&self, files: Vec<InputFile>) {
        glib::MainContext::default().spawn_local(clone!(@weak self as this => async move {
            this.open_success(files).await;
        }));
    }

    pub fn load_clipboard(&self) {
        let clipboard = self.clipboard();
        if clipboard.formats().contain_mime_type("image/png") {
            glib::MainContext::default().spawn_local(clone!(@weak self as this => async move {
                let t = clipboard.read_texture_future().await;
                if let Ok(Some(t)) = t {
                    let interim = JobFile::new(FileType::Png, Some(format!("{}.png",gettext("Pasted Image"))));
                    t.save_to_png(interim.as_filename()).ok();
                    let file = InputFile::new(&gio::File::for_path(interim.as_filename())).unwrap();
                    this.open_success(vec![file]).await;
                }
            }));
        }
    }

    async fn open_success(&self, files: Vec<InputFile>) {
        self.imp().input_file_store.remove_all();

        self.imp().stack.set_visible_child_name("stack_loading");

        for file in files.iter() {
            self.imp().input_file_store.append(file);
        }

        let file_paths = files.iter().map(|f| f.path()).collect_vec();

        let file_paths_pixbuf = files
            .iter()
            .filter(|f| f.kind().supports_pixbuff())
            .take(5)
            .map(|f| f.generate_pixbuff());

        join_all(file_paths_pixbuf).await;

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_LOW);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let jobs = file_paths
                .into_iter()
                .map(|f| async move { count_frames(f).await.unwrap_or(1) })
                .collect_vec();
            sender
                .send(rt.block_on(join_all(jobs)))
                .expect("concurrency failure");
        });

        receiver.attach(
            None,
            clone!(@weak self as this => @default-return Continue(false), move |frames| {
                for (f, frame) in files.iter().zip(frames.iter()) {
                    f.set_frames(*frame);
                }

                this.load_pixbuff_finished();
                Continue(false)
            }),
        );
    }

    fn load_pixbuff_finished(&self) {
        let imp = self.imp();

        let mut loaded_pixbuffs = Vec::new();
        for input_file in self.imp().input_file_store.into_iter().flatten() {
            if let Ok(input_file) = input_file.downcast::<InputFile>() {
                if let Some(pixbuff) = input_file.pixbuf() {
                    loaded_pixbuffs.push(pixbuff);
                }
            }
        }

        let mut image_width = 1000;
        let mut image_height = 1000;

        match loaded_pixbuffs.len() {
            0 => {
                imp.image.set_pixbuf(None);
                imp.image_size_label.set_visible(false);
            }
            1 => {
                image_width = loaded_pixbuffs[0].width();
                image_height = loaded_pixbuffs[0].height();
                imp.image_size_label
                    .set_subtitle(&format!("({} × {})", image_width, image_height));
                imp.image.set_pixbuf(Some(&loaded_pixbuffs[0]));
                imp.image_size_label.set_visible(true);
            }
            _ => {
                fn stack_images(pixbuffs: &Vec<Pixbuf>) -> Pixbuf {
                    let side = pixbuffs
                        .iter()
                        .map(|p| std::cmp::min(p.width(), p.height()))
                        .min()
                        .unwrap();
                    let larger_side =
                        side + (((side as f64) * 0.2 * ((pixbuffs.len() - 1) as f64)) as i32);
                    let canvas = Pixbuf::new(
                        gdk::gdk_pixbuf::Colorspace::Rgb,
                        true,
                        8,
                        larger_side,
                        larger_side,
                    )
                    .unwrap();
                    for (i, pix) in pixbuffs.iter().enumerate() {
                        let offset = ((i as f64) * 0.2 * (side as f64)) as i32;
                        let internal_width = (side as f64) / (pix.width() as f64);
                        let internal_height = (side as f64) / (pix.height() as f64);
                        let internal_offset = match internal_width > internal_height {
                            true => internal_width,
                            false => internal_height,
                        };
                        pix.scale(
                            &canvas,
                            offset,
                            offset,
                            side,
                            side,
                            offset as f64,
                            offset as f64,
                            internal_offset,
                            internal_offset,
                            gtk::gdk_pixbuf::InterpType::Bilinear,
                        );
                    }
                    canvas
                }
                image_width = loaded_pixbuffs[0].width();
                image_height = loaded_pixbuffs[0].height();
                imp.image.set_pixbuf(Some(&stack_images(&loaded_pixbuffs)));
                imp.image_size_label.set_visible(false);
            }
        };

        self.load_options();
        imp.resize_scale_height_value.set_text("100");
        imp.resize_scale_width_value.set_text("100");
        imp.ratio_width_value.set_text("1");
        imp.ratio_height_value.set_text("1");
        imp.resize_width_value.set_text(&image_width.to_string());
        imp.resize_height_value.set_text(&image_height.to_string());
        imp.svg_size_width_value.set_text(&image_width.to_string());
        imp.svg_size_height_value
            .set_text(&image_height.to_string());
        imp.resize_minmax_width_value
            .set_text(&image_width.to_string());
        imp.resize_minmax_height_value
            .set_text(&image_height.to_string());
        self.update_output_options();
        self.update_advanced_options();

        imp.stack.set_visible_child_name("stack_convert");
    }

    pub fn get_selected_output(&self) -> Option<FileType> {
        match self.imp().output_filetype.selected_item() {
            Some(o) => match o.downcast::<gtk::StringObject>() {
                Ok(o) => Some(FileType::from_string(&o.string().as_str().to_lowercase()).unwrap()),
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn get_selected_compression(&self) -> Option<CompressionType> {
        match self.imp().output_compression.selected_item() {
            Some(o) => match o.downcast::<gtk::StringObject>() {
                Ok(o) => {
                    Some(CompressionType::from_string(&o.string().as_str().to_lowercase()).unwrap())
                }
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn update_output_options(&self) {
        let previous_option = self
            .get_selected_output()
            .unwrap_or(self.load_selected_output());

        let new_options = gtk::StringList::new(&[]);
        let new_list = FileType::output_formats(self.imp().settings.boolean("show-less-popular"))
            .collect_vec();
        for ft in new_list.iter() {
            new_options.append(&ft.as_display_string());
        }
        self.imp().output_filetype.set_model(Some(&new_options));
        if let Some(index) = new_list.into_iter().position(|p| p == &previous_option) {
            self.imp().output_filetype.set_selected(index as u32);
        }
        self.update_compression_options();
    }

    pub fn update_compression_options(&self) {
        let files = self
            .imp()
            .input_file_store
            .into_iter()
            .flatten()
            .flat_map(|o| o.downcast::<InputFile>())
            .collect_vec();
        let multiple_files = files.len() > 1;
        let multiple_frames = multiple_files || files.iter().map(|i| i.frames()).sum::<usize>() > 1;
        let output_option = self.get_selected_output().unwrap();
        match (multiple_files, multiple_frames) {
            (false, false) => {
                self.imp().output_compression.set_visible(false);
            }
            (false, true) if output_option.supports_animation() => {
                self.imp().output_compression.set_visible(false);
            }
            _ => {
                let previous_option = self
                    .get_selected_compression()
                    .unwrap_or(self.load_selected_compression());

                let new_options = gtk::StringList::new(&[]);
                let sandboxed = files.iter().any(|f: &InputFile| f.is_behind_sandbox());
                let new_list = CompressionType::possible_output(sandboxed).collect_vec();
                for ct in new_list.iter() {
                    new_options.append(&ct.as_display_string());
                }
                self.imp().output_compression.set_model(Some(&new_options));
                self.imp().output_compression.set_visible(true);

                if let Some(index) = new_list.into_iter().position(|p| *p == previous_option) {
                    self.imp().output_compression.set_selected(index as u32);
                }
            }
        }
    }

    pub fn update_advanced_options(&self) {
        let imp = self.imp();

        let input_files: Vec<InputFile> =
            imp.input_file_store.iter::<InputFile>().flatten().collect();
        let input_filetypes: Vec<FileType> = input_files.iter().map(|inf| inf.kind()).collect();
        let text_filetypes: Vec<String> = input_filetypes
            .iter()
            .unique()
            .map(|ft| {
                format!(
                    "{} ({})",
                    ft.as_extension().to_ascii_uppercase(),
                    ft.as_mime()
                )
            })
            .collect();
        let Some(output_filetype) = FileType::output_formats(self.imp().settings.boolean("show-less-popular")).nth(imp.output_filetype.selected() as usize) else {
            return;
        };

        imp.image_type_label
            .set_subtitle(&text_filetypes.join(", "));
        imp.quality_row.set_visible(false);
        imp.bgcolor_row.set_visible(false);
        imp.resize_row.set_visible(false);
        imp.resize_row.set_enable_expansion(false);
        imp.svg_size_row.set_visible(false);
        imp.svg_size_row.set_enable_expansion(false);
        imp.dpi_row.set_visible(false);

        if output_filetype.is_lossy() {
            imp.quality_row.set_visible(true);
        }

        if input_filetypes
            .iter()
            .any(|input_file| input_file.supports_alpha())
        {
            imp.bgcolor_row.set_visible(true);

            if output_filetype.supports_alpha() {
                // imp.bgcolor.dialog().unwrap().set_with_alpha(true);
                imp.bgcolor.set_rgba(&gdk::RGBA::TRANSPARENT);
                let color_dialog = ColorDialog::new();
                color_dialog.set_with_alpha(true);
                imp.bgcolor.set_dialog(&color_dialog);
            } else {
                // imp.bgcolor.dialog().unwrap().set_wi th_alpha(false);
                imp.bgcolor.set_rgba(&gdk::RGBA::WHITE);
                let color_dialog = ColorDialog::new();
                color_dialog.set_with_alpha(false);
                imp.bgcolor.set_dialog(&color_dialog);
            }
        }

        if input_filetypes
            .iter()
            .any(|input_filetype| *input_filetype == FileType::Svg)
        {
            imp.svg_size_row.set_visible(true);
        }

        if input_filetypes
            .iter()
            .any(|input_filetype| *input_filetype == FileType::Pdf)
        {
            imp.dpi_row.set_visible(true);
        }

        imp.resize_row.set_visible(true);
    }

    fn update_resize(&self) {
        let imp = self.imp();

        // let resize_algorithm = ResizeFilter::from_index(imp.resize_filter.selected() as usize).unwrap();
        let resize_type = ResizeType::from_index(imp.resize_type.selected() as usize).unwrap();
        imp.resize_height_row.set_visible(false);
        imp.resize_width_row.set_visible(false);
        imp.resize_scale_height_row.set_visible(false);
        imp.resize_scale_width_row.set_visible(false);
        imp.ratio_height_row.set_visible(false);
        imp.ratio_width_row.set_visible(false);
        imp.resize_minmax_height_row.set_visible(false);
        imp.resize_minmax_width_row.set_visible(false);

        match resize_type {
            ResizeType::Percentage => {
                imp.resize_scale_width_row.set_visible(true);
                imp.resize_scale_height_row.set_visible(true);
            }
            ResizeType::ExactPixels => {
                imp.resize_width_row.set_visible(true);
                imp.resize_height_row.set_visible(true);
                match imp.resize_width_row.selected() {
                    0 => imp.resize_width_value.set_visible(true),
                    1 => imp.resize_width_value.set_visible(false),
                    _ => unreachable!("Unexpected resize width value"),
                }
                match imp.resize_height_row.selected() {
                    0 => imp.resize_height_value.set_visible(true),
                    1 => imp.resize_height_value.set_visible(false),
                    _ => unreachable!("Unexpected resize width value"),
                }
            }
            ResizeType::MaxPixels | ResizeType::MinPixels => {
                imp.resize_minmax_height_row.set_visible(true);
                imp.resize_minmax_width_row.set_visible(true);
            }
            ResizeType::Ratio => {
                imp.ratio_height_row.set_visible(true);
                imp.ratio_width_row.set_visible(true);
            }
        }
    }

    fn update_svg_size(&self) {
        let imp = self.imp();
        match imp.svg_size_type.selected() {
            0 => {
                imp.svg_size_width_row.set_visible(true);
                imp.svg_size_height_row.set_visible(false);
            }
            1 => {
                imp.svg_size_width_row.set_visible(false);
                imp.svg_size_height_row.set_visible(true);
            }
            _ => unreachable!("Invalid SVG resize value"),
        }
    }

    pub fn open_files(&self, files: Vec<Option<InputFile>>) {
        let files: Vec<InputFile> = files.into_iter().flatten().collect();
        if !files.is_empty() {
            self.open_success_wrapper(files);
        } else {
            self.imp()
                .stack
                .set_visible_child_name("stack_welcome_page");
        }
    }

    fn save_error(&self, error: Option<&str>) {
        if let Some(s) = error {
            self.show_toast(s);
        }
    }

    pub fn save_files(&self) {
        let files = self
            .imp()
            .input_file_store
            .into_iter()
            .flatten()
            .flat_map(|o| o.downcast::<InputFile>())
            .collect_vec();
        let multiple_files = files.len() > 1;
        let multiple_frames = multiple_files || files.iter().map(|i| i.frames()).sum::<usize>() > 1;
        let output_option = self.get_selected_output().unwrap();
        let first_file_path = files.first().unwrap().path();
        let first_file_path = std::path::Path::new(&first_file_path);
        let (save_format, default_name) = match (multiple_files, multiple_frames) {
            (false, false) => {
                let file_stem = first_file_path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();

                (OutputType::File(output_option), file_stem)
            }
            (false, true) if output_option.supports_animation() => {
                let file_stem = first_file_path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();

                (OutputType::File(output_option), file_stem)
            }
            _ => (
                OutputType::Compression(self.get_selected_compression().unwrap()),
                "images".to_owned(),
            ),
        };

        let sandboxed = files.iter().any(|f: &InputFile| f.is_behind_sandbox());

        let default_folder = match sandboxed {
            true => None,
            false => Some(
                first_file_path
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            ),
        };

        if save_format != OutputType::Compression(CompressionType::Directory) {
            FileChooser::choose_output_file_wrapper(
                self,
                format!("{default_name}.{}", save_format.as_extension()),
                save_format,
                default_folder,
                AppWindow::save_success_wrapper,
                AppWindow::save_error,
            );
        } else {
            FileChooser::choose_output_folder_wrapper(
                self,
                default_folder,
                AppWindow::save_success_wrapper,
                AppWindow::save_error,
            );
        }
    }

    fn save_success_wrapper(&self, save_format: OutputType, path: String) {
        glib::MainContext::default().spawn_local(clone!(@weak self as this => async move {
            this.save_success(save_format, path).await;
        }));
    }

    fn get_quality_argument(&self) -> usize {
        self.imp().quality.value() as usize
    }

    fn get_dpi_argument(&self) -> usize {
        self.imp().dpi_value.text().parse().unwrap()
    }

    fn get_bgcolor_argument(&self) -> Color {
        self.imp().bgcolor.rgba().into()
    }

    fn get_filter_argument(&self) -> Option<ResizeFilter> {
        match self.imp().resize_row.is_expanded() {
            true => ResizeFilter::from_index(self.imp().resize_filter.selected() as usize),
            false => None,
        }
    }

    fn get_svg_size_argument(&self) -> Option<SizeArgument> {
        let imp = self.imp();

        match (imp.svg_size_row.is_expanded(), imp.svg_size_type.selected()) {
            (true, 0) => Some(SizeArgument::Width(
                imp.svg_size_width_value
                    .text()
                    .as_str()
                    .to_owned()
                    .parse()
                    .unwrap(),
            )),
            (true, 1) => Some(SizeArgument::Height(
                imp.svg_size_width_value
                    .text()
                    .as_str()
                    .to_owned()
                    .parse()
                    .unwrap(),
            )),
            _ => None,
        }
    }

    fn get_resize_argument(&self) -> Option<ResizeArgument> {
        let imp = self.imp();

        if !imp.resize_row.is_expanded() {
            return None;
        }

        let resize_type = ResizeType::from_index(imp.resize_type.selected() as usize).unwrap();

        match resize_type {
            ResizeType::Percentage => Some(ResizeArgument::Percentage {
                width: imp
                    .resize_scale_width_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
                height: imp
                    .resize_scale_height_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
            ResizeType::ExactPixels => Some(ResizeArgument::ExactPixels {
                width: match imp.resize_width_row.selected() {
                    0 => Some(imp.resize_width_value.text().to_string().parse().unwrap()),
                    _ => None,
                },
                height: match imp.resize_height_row.selected() {
                    0 => Some(imp.resize_height_value.text().to_string().parse().unwrap()),
                    _ => None,
                },
            }),
            ResizeType::MaxPixels => Some(ResizeArgument::MaxPixels {
                width: imp
                    .resize_minmax_width_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
                height: imp
                    .resize_minmax_height_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
            ResizeType::MinPixels => Some(ResizeArgument::MinPixels {
                width: imp
                    .resize_minmax_width_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
                height: imp
                    .resize_minmax_height_value
                    .text()
                    .to_string()
                    .parse()
                    .unwrap(),
            }),
            ResizeType::Ratio => Some(ResizeArgument::Ratio {
                width: imp.ratio_width_value.text().to_string().parse().unwrap(),
                height: imp.ratio_height_value.text().to_string().parse().unwrap(),
            }),
        }
    }

    async fn save_success(&self, save_format: OutputType, path: String) {
        use FileType::*;

        self.imp().convert_button.set_sensitive(false);
        self.imp().progress_bar.set_text(Some(&gettext("Loading…")));
        self.imp().progress_bar.set_fraction(0.0);
        self.imp()
            .is_canceled
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.imp().current_jobs.replace(vec![]);
        self.save_options().ok();
        self.save_selected_output().ok();
        self.save_selected_compression().ok();

        let output_type = self.get_selected_output().unwrap();

        let files = self
            .imp()
            .input_file_store
            .into_iter()
            .flatten()
            .flat_map(|o| o.downcast::<InputFile>())
            .collect_vec();

        let dir = create_temporary_dir().await.unwrap();

        fn generate_job(
            input_path: &str,
            frame: usize,
            input_type: &FileType,
            output_path: &str,
            output_type: &FileType,
            dir: &TempDir,
            default_arguments: (&MagickConvertJob, &GhostScriptConvertJob),
        ) -> Vec<ConvertJob> {
            match (input_type, output_type) {
                (Svg, Heif | Heic) => {
                    let interm = get_temp_file_path(dir, JobFile::new(FileType::Png, None))
                        .to_str()
                        .unwrap()
                        .to_owned();
                    generate_job(
                        input_path,
                        frame,
                        input_type,
                        &interm,
                        &FileType::Png,
                        dir,
                        default_arguments,
                    )
                    .into_iter()
                    .chain(
                        generate_job(
                            &interm,
                            0,
                            &FileType::Png,
                            output_path,
                            output_type,
                            dir,
                            default_arguments,
                        )
                        .into_iter(),
                    )
                    .collect()
                }
                (Pdf, Png) => std::iter::once(ConvertJob::GhostScript(GhostScriptConvertJob {
                    input_file: input_path.to_owned(),
                    output_file: output_path.to_owned(),
                    page: frame,
                    ..*default_arguments.1
                }))
                .collect(),
                (Pdf, _) => {
                    let interm = get_temp_file_path(dir, JobFile::new(FileType::Png, None))
                        .to_str()
                        .unwrap()
                        .to_owned();
                    generate_job(
                        input_path,
                        frame,
                        input_type,
                        &interm,
                        &FileType::Png,
                        dir,
                        default_arguments,
                    )
                    .into_iter()
                    .chain(
                        generate_job(
                            &interm,
                            0,
                            &FileType::Png,
                            output_path,
                            output_type,
                            dir,
                            default_arguments,
                        )
                        .into_iter(),
                    )
                    .collect()
                }
                (Gif, Webp) | (Webp, Gif) => {
                    std::iter::once(ConvertJob::Magick(MagickConvertJob {
                        input_file: input_path.to_owned(),
                        output_file: output_path.to_owned(),
                        first_frame: false,
                        coalesce: false,
                        size_arg: None,
                        ..*default_arguments.0
                    }))
                    .collect()
                }
                _ => std::iter::once(ConvertJob::Magick(MagickConvertJob {
                    input_file: input_path.to_owned(),
                    output_file: output_path.to_owned(),
                    first_frame: true,
                    coalesce: false,
                    ..*default_arguments.0
                }))
                .collect(),
            }
        }

        let job_input = files
            .into_iter()
            .map(|f| {
                let path = f.path();
                let path = Path::new(&path);
                let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
                let re = regex::Regex::new(r"_\d\d*$").unwrap();
                let stripped_stem = re.replace(&stem, "").to_string();
                (f, stripped_stem)
            })
            .sorted_by(|(_, s1), (_, s2)| std::cmp::Ord::cmp(s1, s2))
            .group_by(|(_, s)| s.to_owned())
            .into_iter()
            .flat_map(|(_, fs)| {
                fs.enumerate().map(|(i, (f, s))| match i {
                    0 => (f, s),
                    x => (f, format!("{s}_{x}")),
                })
            })
            .flat_map(|(f, output_stem)| {
                let (path, input_filetype, frames) = (f.path(), f.kind(), f.frames());
                let path = Path::new(&path);
                let parent = path.parent().unwrap().to_str().unwrap();
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let input_ext = path.extension().unwrap().to_str().unwrap();
                match (input_filetype, output_type, frames) {
                    (_, _, 0) => unreachable!("an image cannot have zero frames"),
                    (Pdf, _, c) => (0..c)
                        .map(|f| {
                            (
                                format!("{parent}/{stem}.{input_ext}"),
                                input_filetype,
                                format!("{output_stem}_{f}.{}", output_type.as_extension()),
                                f,
                            )
                        })
                        .collect_vec(),
                    (_, _, 1) => vec![(
                        format!("{parent}/{stem}.{input_ext}[0]"),
                        input_filetype,
                        format!("{output_stem}.{}", output_type.as_extension()),
                        0,
                    )],
                    (Webp | Gif, Webp | Gif, _) => vec![(
                        format!("{parent}/{stem}.{input_ext}"),
                        input_filetype,
                        format!("{output_stem}.{}", output_type.as_extension()),
                        0,
                    )],
                    (Webp | Gif, _, count) => (0..count)
                        .map(|f| {
                            (
                                format!("{parent}/{stem}.{input_ext}[{f}]"),
                                input_filetype,
                                format!("{output_stem}_{f}.{}", output_type.as_extension()),
                                f,
                            )
                        })
                        .collect_vec(),
                    _ => vec![(
                        format!("{parent}/{stem}.{input_ext}[0]"),
                        input_filetype,
                        format!("{output_stem}.{}", output_type.as_extension()),
                        0,
                    )],
                }
            })
            .collect_vec();

        let output_files = job_input
            .iter()
            .map(|(_, _, o, _)| {
                get_temp_file_path(&dir, JobFile::new(output_type, Some(o.to_string())))
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect_vec();

        let magick_arguments = MagickConvertJob {
            input_file: "".to_string(),
            output_file: "".to_string(),
            background: self.get_bgcolor_argument(),
            quality: self.get_quality_argument(),
            filter: self.get_filter_argument(),
            size_arg: self.get_svg_size_argument(),
            resize_arg: self.get_resize_argument(),
            coalesce: false,
            first_frame: false,
        };

        let ghost_arguments = GhostScriptConvertJob {
            input_file: "".to_string(),
            output_file: "".to_string(),
            page: 0,
            dpi: self.get_dpi_argument(),
        };

        let magick_jobs = job_input
            .into_iter()
            .map(|(f, ft, os, frame)| {
                generate_job(
                    &f,
                    frame,
                    &ft,
                    get_temp_file_path(&dir, JobFile::new(output_type, Some(os)))
                        .to_str()
                        .unwrap(),
                    &output_type,
                    &dir,
                    (&magick_arguments, &ghost_arguments),
                )
            })
            .collect_vec();

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let count = magick_jobs.iter().map(|mjs| mjs.len()).sum();

        let completed = std::sync::Arc::new(AtomicUsize::new(0));

        let stop_flag = self.imp().is_canceled.clone();
        let stop_flag_s = stop_flag.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            let stop_flag = stop_flag_s.clone();

            fdlimit::raise_fd_limit();

            let jobs = magick_jobs
                .into_iter()
                .map(|mjs| {
                    let stop_flag = stop_flag.clone();
                    let sender = sender.clone();
                    async move {
                        spawn(async move {
                            for mut mj_command in mjs.into_iter().map(|mj| mj.get_command()) {
                                if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                    return;
                                }
                                let std::io::Result::Ok(shared_child) = SharedChild::spawn(&mut mj_command) else {
                                    dbg!("panic");
                                    sender.send(ArcOrOptionError::OptionError(Some("cannot generate command".to_string()))).expect("Concurrency Issue");
                                    return;
                                };
                                if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                    return;
                                }
                                let child_arc = std::sync::Arc::new(shared_child);
                                sender
                                    .send(ArcOrOptionError::Child(child_arc.clone()))
                                    .expect("Concurrency Issue");
                                let output = match wait_for_child(child_arc.clone()).await {
                                    Ok(_) => None,
                                    Err(e) => Some(e),
                                };
                                if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                    return;
                                }

                                sender
                                    .send(ArcOrOptionError::OptionError(output))
                                    .expect("Concurrency Issue");
                            }
                        })
                        .await.ok();
                    }
                })
                .collect_vec();

            rt.block_on(join_all(jobs));
        });

        let dir_path = dir.path().to_str().unwrap().to_string();

        std::mem::forget(dir);

        let stop_flag_r = stop_flag;

        receiver.attach(
            None,
            clone!(@weak self as this => @default-return Continue(false), move |e| {
                match e {
                    ArcOrOptionError::Child(c) => {
                        if stop_flag_r.load(std::sync::atomic::Ordering::SeqCst) {
                            match c.kill() {
                                Ok(_) => {}
                                Err(_) => {c.wait().ok();}
                            }
                        } else {
                            this.imp().current_jobs.borrow_mut().push(c);
                        }
                        Continue(true)
                    }
                    ArcOrOptionError::OptionError(e) => {
                        if let Some(e) = e {
                            stop_flag_r.store(true, std::sync::atomic::Ordering::SeqCst);
                            this.convert_failed(e, dir_path.clone());
                            return Continue(false);
                        }
                        let c = completed.clone();
                        let x = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        this.set_convert_progress(x + 1, count);
                        if x + 1 == count {
                            this.move_output(save_format, path.clone(), output_files.clone(), dir_path.clone());
                            Continue(false)
                        } else {
                            Continue(true)
                        }
                    }
                }
            }),
        );

        self.imp().stack.set_visible_child_name("stack_converting");
    }

    fn move_output(
        &self,
        save_format: OutputType,
        path: String,
        output_files: Vec<String>,
        dir_path: String,
    ) {
        let stop_flag = self.imp().is_canceled.clone();
        if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let path_r = path.clone();

        self.set_collecting_progress();
        let receiver = match save_format {
            OutputType::File(_) => {
                let file = output_files.first().unwrap().to_owned();

                let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    let shared_child: SharedChild =
                        SharedChild::spawn(std::process::Command::new("mv").arg(file).arg(path))
                            .unwrap();
                    let child_arc = std::sync::Arc::new(shared_child);

                    sender
                        .send(ArcOrOptionError::Child(child_arc.clone()))
                        .expect("Concurrency Issues");

                    sender
                        .send(ArcOrOptionError::OptionError(
                            match rt.block_on(wait_for_child(child_arc)) {
                                Err(e) => Some(e),
                                _ => None,
                            },
                        ))
                        .expect("Concurrency Issues");
                });

                receiver
            }
            OutputType::Compression(CompressionType::Directory) => {
                let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    let shared_child: SharedChild = SharedChild::spawn(
                        std::process::Command::new("mv")
                            .args(output_files)
                            .arg(path),
                    )
                    .unwrap();
                    let child_arc = std::sync::Arc::new(shared_child);

                    sender
                        .send(ArcOrOptionError::Child(child_arc.clone()))
                        .expect("Concurrency Issues");

                    sender
                        .send(ArcOrOptionError::OptionError(
                            match rt.block_on(wait_for_child(child_arc)) {
                                Err(e) => Some(e),
                                _ => None,
                            },
                        ))
                        .expect("Concurrency Issues");
                });

                receiver
            }
            _ => {
                let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    let shared_child: SharedChild = SharedChild::spawn(
                        std::process::Command::new("zip")
                            .arg("-jFSm0")
                            .arg(path)
                            .args(output_files),
                    )
                    .unwrap();
                    let child_arc = std::sync::Arc::new(shared_child);

                    sender
                        .send(ArcOrOptionError::Child(child_arc.clone()))
                        .expect("Concurrency Issues");

                    sender
                        .send(ArcOrOptionError::OptionError(
                            match rt.block_on(wait_for_child(child_arc)) {
                                Err(e) => Some(e),
                                _ => None,
                            },
                        ))
                        .expect("Concurrency Issues");
                });

                receiver
            }
        };

        receiver.attach(
            None,
            clone!(@weak self as this => @default-return Continue(false), move |x| {
                match x {
                    ArcOrOptionError::Child(c) => {
                        if this.imp().is_canceled.load(std::sync::atomic::Ordering::SeqCst) {
                            match c.kill() {
                                Ok(_) => {}
                                Err(_) => {c.wait().ok();}
                            }
                        } else {
                            this.imp().current_jobs.borrow_mut().push(c);
                        }
                        Continue(true)
                    }
                    ArcOrOptionError::OptionError(x) => {
                        match x {
                            Some(e) => this.convert_failed(e, dir_path.clone()),
                            None => this.convert_success(dir_path.clone(), path_r.clone(), save_format)
                        }
                        Continue(false)
                    }
                }
            }),
        );
    }

    fn convert_failed(&self, error_message: String, temp_dir_path: String) {
        let mut current_jobs = self.imp().current_jobs.borrow_mut();
        for x in current_jobs.iter() {
            match x.kill() {
                Ok(_) => {}
                Err(_) => {
                    x.wait().ok();
                }
            }
        }
        current_jobs.clear();

        let dialog =
            adw::MessageDialog::new(Some(self), Some(&gettext("Error while processing")), None);

        let sw = gtk::ScrolledWindow::new();
        sw.set_min_content_height(200);
        sw.set_max_content_height(400);
        sw.add_css_class("card");

        let text = gtk::Label::new(Some(&error_message));
        text.set_margin_top(12);
        text.set_margin_bottom(12);
        text.set_margin_start(12);
        text.set_margin_end(12);
        text.set_xalign(0.0);
        text.set_yalign(0.0);
        text.add_css_class("monospace");
        text.set_wrap(true);
        text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        sw.set_child(Some(&text));
        dialog.set_extra_child(Some(&sw));

        dialog.add_responses(&[
            ("copy", &gettext("_Copy to clipboard")),
            ("ok", &gettext("_Dismiss")),
        ]);
        dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);
        dialog.connect_response(
            None,
            clone!(@weak self as this => move |d, response_id| {
                if response_id == "copy" {
                    this.clipboard().set_text(&error_message);
                    this.show_toast(&gettext("Error copied to clipboard"));
                }
                d.close();
            }),
        );
        dialog.present();

        self.imp().stack.set_visible_child_name("stack_convert");
        self.convert_clean(temp_dir_path);
    }

    fn convert_success(&self, temp_dir_path: String, path: String, save_format: OutputType) {
        self.convert_clean(temp_dir_path);
        let toast = adw::Toast::new(&gettext("Image converted"));
        toast.set_button_label(Some(&gettext("Open")));
        toast.connect_button_clicked(clone!(@weak self as this => move |_| {
            let p = path.clone();
            glib::MainContext::default().spawn_local(clone!(@weak this as other_this => async move {
                match save_format {
                    OutputType::Compression(CompressionType::Directory) => {
                        ashpd::desktop::open_uri::OpenDirectoryRequest::default().send(&std::fs::File::open(&p).unwrap()).await.ok();
                    }
                    _ => {
                        ashpd::desktop::open_uri::OpenFileRequest::default().ask(true).send_file(&std::fs::File::open(&p).unwrap()).await.ok();
                    }
                }
            }));
        }));
        self.imp().toast_overlay.add_toast(toast);
        self.imp().stack.set_visible_child_name("stack_convert");
    }

    fn convert_clean(&self, temp_dir_path: String) {
        clean_dir(temp_dir_path);
        self.imp().convert_button.set_sensitive(true);
        // self.imp().progress_bar.set_text(Some(&gettext("Loading…")));
        // self.imp().progress_bar.set_fraction(0.0);
    }

    fn convert_cancel(&self) {
        let stop_converting_dialog = adw::MessageDialog::new(
            Some(self),
            Some(&gettext("Stop converting?")),
            Some(&gettext("You will lose all progress.")),
        );

        stop_converting_dialog
            .add_responses(&[("cancel", &gettext("_Cancel")), ("stop", &gettext("_Stop"))]);
        stop_converting_dialog
            .set_response_appearance("stop", adw::ResponseAppearance::Destructive);

        stop_converting_dialog.connect_response(
            None,
            clone!(@weak self as this => move |_, response_id| {
                if response_id == "stop" {
                    this.imp()
                        .is_canceled
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    let mut current_jobs = this.imp().current_jobs.borrow_mut();
                    for x in current_jobs.iter() {
                        match x.kill() {
                            Ok(_) => {}
                            Err(_) => {
                                x.wait().ok();
                            }
                        }
                    }
                    current_jobs.clear();
                    this.imp().stack.set_visible_child_name("stack_convert");
                    this.show_toast(&gettext("Converting Cancelled"));
                }
            }),
        );

        stop_converting_dialog.present();
    }
}
