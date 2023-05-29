use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

use crate::color::Color;
use crate::config::APP_ID;
use crate::drag_overlay::DragOverlay;
use crate::file_chooser::FileChooser;
use crate::filetypes::{CompressionType, FileType, OutputType};
use crate::input_file::InputFile;
use crate::magick::{
    count_frames, generate_job, wait_for_child, GhostScriptConvertJob, JobFile, MagickConvertJob,
    ResizeArgument,
};
use crate::temp::{clean_dir, create_temporary_dir, get_temp_file_path};
use crate::widgets::about_window::ConverterAbout;
use crate::widgets::image_rest::ImageRest;
use crate::widgets::image_thumbnail::ImageThumbnail;
use adw::prelude::*;
use futures::future::join_all;
use gettextrs::gettext;
use glib::{clone, idle_add_local_once, MainContext};
use gtk::accessible::Property;
use gtk::gdk::Texture;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::{gdk, gio, glib, subclass::prelude::*};
use itertools::Itertools;
use shared_child::SharedChild;
use std::sync::Arc;
use tokio::spawn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFilter {
    Default,
    Point,
}

enum ArcOrOptionError {
    Child(Arc<SharedChild>),
    OptionError(Option<String>),
}

#[allow(dead_code)]
impl ResizeFilter {
    pub fn as_display_string(&self) -> Option<&str> {
        match self {
            ResizeFilter::Default => None,
            ResizeFilter::Point => Some("Point"),
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ResizeFilter::Default),
            1 => Some(ResizeFilter::Point),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ResizeType {
    Percentage,
    ExactPixels,
}

impl ResizeType {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(ResizeType::Percentage),
            1 => Some(ResizeType::ExactPixels),
            _ => None,
        }
    }
}

mod imp {
    use std::{
        cell::{Cell, RefCell},
        sync::atomic::AtomicBool,
    };

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
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub all_images_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub open_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub other_add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub convert_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub loading_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub loading_spinner_images: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub image_container: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub full_image_container: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub supported_output_filetypes: TemplateChild<gtk::StringList>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        pub output_filetype: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub output_compression: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub output_compression_value: TemplateChild<gtk::Switch>,

        #[template_child]
        pub quality: TemplateChild<gtk::Scale>,
        #[template_child]
        pub bgcolor: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub resize_filter_default: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub resize_filter_pixel: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub resize_filter_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_amount_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub resize_type: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub resize_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub link_axis: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub resize_scale_width_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub resize_scale_height_value: TemplateChild<gtk::Entry>,
        #[template_child]
        pub dpi_value: TemplateChild<gtk::Entry>,

        #[template_child]
        pub quality_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub bgcolor_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub dpi_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub leaf: TemplateChild<adw::Leaflet>,

        pub provider: gtk::CssProvider,
        pub input_file_store: gio::ListStore,
        pub settings: gio::Settings,
        pub is_canceled: std::sync::Arc<AtomicBool>,
        pub current_jobs: RefCell<Vec<Arc<SharedChild>>>,
        pub image_width: Cell<Option<u32>>,
        pub image_height: Cell<Option<u32>>,
        pub removed: RefCell<HashSet<u32>>,
        pub elements: Cell<usize>,
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
                stack: TemplateChild::default(),
                all_images_stack: TemplateChild::default(),
                open_button: TemplateChild::default(),
                add_button: TemplateChild::default(),
                other_add_button: TemplateChild::default(),
                back_button: TemplateChild::default(),
                convert_button: TemplateChild::default(),
                cancel_button: TemplateChild::default(),
                loading_spinner: TemplateChild::default(),
                loading_spinner_images: TemplateChild::default(),
                image_container: TemplateChild::default(),
                full_image_container: TemplateChild::default(),
                supported_output_filetypes: TemplateChild::default(),
                progress_bar: TemplateChild::default(),
                output_filetype: TemplateChild::default(),
                output_compression: TemplateChild::default(),
                output_compression_value: TemplateChild::default(),
                quality: TemplateChild::default(),
                bgcolor: TemplateChild::default(),
                resize_filter_default: TemplateChild::default(),
                resize_filter_pixel: TemplateChild::default(),
                resize_filter_row: TemplateChild::default(),
                resize_amount_row: TemplateChild::default(),
                resize_type: TemplateChild::default(),
                resize_width_value: TemplateChild::default(),
                resize_height_value: TemplateChild::default(),
                link_axis: TemplateChild::default(),
                resize_scale_width_value: TemplateChild::default(),
                resize_scale_height_value: TemplateChild::default(),
                dpi_value: TemplateChild::default(),
                quality_row: TemplateChild::default(),
                bgcolor_row: TemplateChild::default(),
                dpi_row: TemplateChild::default(),
                leaf: TemplateChild::default(),
                provider: gtk::CssProvider::new(),
                input_file_store: gio::ListStore::new(InputFile::static_type()),

                settings: gio::Settings::new(APP_ID),
                is_canceled: std::sync::Arc::new(AtomicBool::new(true)),
                current_jobs: RefCell::new(Vec::new()),
                image_height: Cell::new(None),
                image_width: Cell::new(None),
                removed: RefCell::new(HashSet::new()),
                elements: Cell::new(0),
            }
        }
    }

    impl ObjectImpl for AppWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.load_window_size();
            obj.setup_gactions();
        }
    }

    impl WidgetImpl for AppWindow {}
    impl WindowImpl for AppWindow {
        fn close_request(&self) -> gtk::Inhibit {
            if let Err(err) = self.obj().save_window_size() {
                dbg!("Failed to save window state, {}", &err);
            }

            if !self.is_canceled.load(std::sync::atomic::Ordering::SeqCst) {
                self.obj().close_dialog();
                glib::signal::Inhibit(true)
            } else {
                // Pass close request on to the parent
                self.parent_close_request()
            }
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
        win.setup_drop_target();

        win
    }

    /// Shows a basic toast with the given text.
    fn show_toast(&self, text: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(text));
    }

    fn setup_gactions(&self) {
        self.add_action_entries([
            gio::ActionEntry::builder("close")
                .activate(clone!(@weak self as window => move |_,_, _| {
                    window.destroy();
                }))
                .build(),
            gio::ActionEntry::builder("add")
                .activate(clone!(@weak self as window => move |_, _, _| {
                    window.add_dialog();
                }))
                .build(),
            gio::ActionEntry::builder("clear")
                .activate(clone!(@weak self as window => move |_, _, _| {
                    window.clear();
                }))
                .build(),
            gio::ActionEntry::builder("about")
                .activate(clone!(@weak self as window => move |_, _, _| {
                    window.show_about();
                }))
                .build(),
            gio::ActionEntry::builder("paste")
                .activate(clone!(@weak self as window => move |_, _, _| {
                    window.load_clipboard();
                }))
                .build(),
        ]);
    }

    fn setup_callbacks(&self) {
        //load imp
        let imp = self.imp();
        imp.open_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.add_dialog();
            }));
        imp.add_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.add_dialog();
            }));
        imp.other_add_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.add_dialog();
            }));
        imp.convert_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.save_files();
            }));
        imp.cancel_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.convert_cancel();
            }));
        imp.output_filetype
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_advanced_options();
                this.update_compression_options();
                this.update_resize();
            }));
        imp.resize_type
            .connect_selected_notify(clone!(@weak self as this => move |_| {
                this.update_resize();
            }));
        imp.resize_width_value
            .connect_changed(clone!(@weak self as this => move |_| {
                this.update_height_from_width();
            }));
        imp.resize_height_value
            .connect_changed(clone!(@weak self as this => move |_| {
                this.update_width_from_height();
            }));
        imp.link_axis
            .connect_clicked(clone!(@weak self as this => move |_| {
                if this.imp().link_axis.is_active() && this.imp().link_axis.is_visible() {
                    this.imp().link_axis.set_icon_name("chain-link-symbolic");
                    let old_value = this.imp().resize_scale_width_value.text().as_str().to_owned();
                    let new_value = this.imp().resize_scale_height_value.text().as_str().to_owned();
                    if old_value != new_value && !new_value.is_empty() {
                        this.imp().resize_scale_width_value.set_text(&new_value);
                    }
                    this.update_width_from_height();
                } else {
                    this.imp().link_axis.set_icon_name("chain-link-loose-symbolic");
                }
            }));

        imp.resize_scale_height_value
            .connect_changed(clone!(@weak self as this => move |_| {
                if this.imp().link_axis.is_active() && this.imp().link_axis.is_visible() {
                    let old_value = this.imp().resize_scale_width_value.text().as_str().to_owned();
                    let new_value = this.imp().resize_scale_height_value.text().as_str().to_owned();
                    if old_value != new_value && !new_value.is_empty() {
                        this.imp().resize_scale_width_value.set_text(&new_value);
                    }
                }
            }));

        imp.resize_scale_width_value
            .connect_changed(clone!(@weak self as this => move |_| {
                if this.imp().link_axis.is_active() && this.imp().link_axis.is_visible() {
                    let old_value = this.imp().resize_scale_height_value.text().as_str().to_owned();
                    let new_value = this.imp().resize_scale_width_value.text().as_str().to_owned();
                    if old_value != new_value && !new_value.is_empty() {
                        this.imp().resize_scale_height_value.set_text(&new_value);
                    }
                }
            }));
        imp.back_button
            .connect_clicked(clone!(@weak self as this => move |_| {
                this.switch_to_stack_convert();
            }));
        imp.image_container.set_filter_func(clone!(@weak self as this => @default-return false, move |f| {
            return (f.index() as usize) >= this.imp().elements.get() || !this.imp().removed.borrow().contains(&(f.index() as u32));
        }));
        imp.full_image_container.set_filter_func(
            clone!(@weak self as this => @default-return false, move |f| {
                return !this.imp().removed.borrow().contains(&(f.index() as u32));
            }),
        );
        imp.resize_filter_default
            .connect_toggled(clone!(@weak self as this => move |f| {
                match (f.is_active(), this.imp().resize_filter_pixel.is_active()) {
                    (x, y) if x == y => {
                        this.imp().resize_filter_pixel.set_active(!x);
                    }
                    _ => {}
                }
            }));
        imp.resize_filter_pixel
            .connect_toggled(clone!(@weak self as this => move |f| {
                match (f.is_active(), this.imp().resize_filter_default.is_active()) {
                    (x, y) if x == y => {
                        this.imp().resize_filter_default.set_active(!x);
                    }
                    _ => {}
                }
            }));
        imp.bgcolor
            .connect_rgba_notify(clone!(@weak self as this => move |x| {
                x.update_property(&[Property::Label(&gettext!("New transparency layer color: {}", x.rgba()))]);
            }));
        self.load_options();
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

                    let input_files = file_list.files().iter().map(InputFile::new).collect_vec();
                    win.open_files(input_files);
                    return true;
                }

                false
            }),
        );

        self.imp().drag_overlay.set_drop_target(&drop_target);
    }

    fn show_about(&self) {
        ConverterAbout::show(self);
    }

    fn close_dialog(&self) {
        let stop_converting_dialog = adw::MessageDialog::new(
            Some(self),
            Some(&gettext("Stop converting?")),
            Some(&gettext("You will lose all progress.")),
        );

        stop_converting_dialog.add_response("cancel", &gettext("_Cancel"));
        stop_converting_dialog.add_response("stop", &gettext("_Stop"));
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
                    this.close();
                }
            }),
        );
        stop_converting_dialog.present();
    }

    fn set_convert_progress(&self, done: usize, total: usize) {
        let msg = gettext!("{}/{}", done, total);
        self.imp().progress_bar.set_text(Some(&msg));
        self.imp()
            .progress_bar
            .set_fraction((done as f64) / (total as f64));
    }

    fn set_collecting_progress(&self) {
        let msg = gettext("Collecting files");
        self.imp().progress_bar.set_text(Some(&msg));
    }

    pub fn load_clipboard(&self) {
        let clipboard = self.clipboard();
        if clipboard.formats().contain_mime_type("image/png") {
            MainContext::default().spawn_local(clone!(@weak self as this => async move {
                let t = clipboard.read_texture_future().await;
                if let Ok(Some(t)) = t {
                    let interim = JobFile::new(FileType::Png, Some(format!("{}.png",gettext("Pasted Image"))));
                    t.save_to_png(interim.as_filename()).ok();
                    let file = InputFile::new(&gio::File::for_path(interim.as_filename())).unwrap();
                    this.open_success(vec![file]).await;
                }
            }));
        } else if clipboard
            .formats()
            .contain_mime_type("application/vnd.portal.files")
        {
            MainContext::default().spawn_local(clone!(@weak self as this => async move {
                let t = clipboard.read_text_future().await.unwrap().unwrap();
                let files = t.lines().flat_map(|p| InputFile::new(&gio::File::for_path(p))).collect();
                this.open_success(files).await;
            }));
        }
    }

    async fn open_success(&self, mut files: Vec<InputFile>) {
        let prev_files = self.active_files();
        files = files.into_iter().chain(prev_files.into_iter()).collect();

        self.imp().input_file_store.remove_all();
        self.imp().removed.replace(HashSet::new());

        self.switch_to_stack_loading_generally();
        // self.switch_to_stack_loading();

        for file in files.iter() {
            self.imp().input_file_store.append(file);
        }

        fdlimit::raise_fd_limit();

        self.load_pixbuf();
    }

    fn load_frames(&self) {
        let files = self.files();
        let file_paths = files.iter().map(|f| f.path()).collect_vec();

        let (sender, receiver) = MainContext::channel(glib::PRIORITY_LOW);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let jobs = file_paths
                .into_iter()
                .map(|f| async move { count_frames(f).await.unwrap_or((1, None)) })
                .collect_vec();

            let res = rt.block_on(join_all(jobs));

            sender.send(res).expect("concurrency failure");
        });

        receiver.attach(
            None,
            clone!(@weak self as this => @default-return Continue(false), move |image_info| {
                let real_files = files.clone();
                for (f, (frame, dims)) in real_files.iter().zip(image_info.iter()) {
                    f.set_frames(*frame);
                    let dims = *dims;
                    idle_add_local_once(clone!(@weak f as ff => move || {
                        if let Some((width, height)) = dims {
                            ff.set_width(width);
                            ff.set_height(height);
                        }
                    }));
                    glib::MainContext::default().iteration(true);
                }
                idle_add_local_once(clone!(@weak this as these => move || {
                    these.load_pixbuf_finished();
                }));


                Continue(false)
            }),
        );
    }

    fn remove_file(&self, i: u32) {
        self.imp().removed.borrow_mut().insert(i);
        if self.files_count() == 0 {
            self.clear();
        } else {
            self.construct_short_thumbnail();
            self.update_options();
        }
    }

    pub fn clear(&self) {
        self.imp().input_file_store.remove_all();

        self.switch_to_stack_welcome();
    }

    fn construct_short_thumbnail(&self) {
        let imp = self.imp();

        let input_files_count = self.files().len();

        let mut elements = 0;
        let mut visible = 0;

        while visible < 6 && elements < input_files_count {
            if !imp.removed.borrow().contains(&(elements as u32)) {
                visible += 1;
            }
            elements += 1;
        }

        let mut remaining_visible = false;

        let mut remaining_elements = elements;

        while !remaining_visible && remaining_elements < input_files_count {
            if !imp.removed.borrow().contains(&(remaining_elements as u32)) {
                remaining_visible = true;
            }
            remaining_elements += 1;
        }

        if remaining_visible {
            elements -= 1;
        }

        self.update_image_container(elements, remaining_visible);
    }

    fn active_files(&self) -> Vec<InputFile> {
        let removed = self.imp().removed.borrow().clone();
        self.files()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !removed.contains(&(*i as u32)))
            .map(|(_, f)| f)
            .collect_vec()
    }

    fn files(&self) -> Vec<InputFile> {
        self.imp()
            .input_file_store
            .iter::<InputFile>()
            .flatten()
            .collect_vec()
    }

    fn load_pixbuf_finished(&self) {
        let imp = self.imp();

        let files_dims = self
            .active_files()
            .into_iter()
            .map(|f| f.dimensions())
            .unique()
            .collect_vec();

        if let Some((w, h)) = match files_dims[..] {
            [Some(d)] => Some(d),
            _ => None,
        } {
            imp.image_width.set(Some(w as u32));
            imp.image_height.set(Some(h as u32));
        } else {
            imp.image_width.set(None);
            imp.image_height.set(None);
        }

        self.construct_short_thumbnail();

        idle_add_local_once(clone!(@weak self as that => move || {
            that.update_full_image_container();
        }));

        self.update_options();
        self.switch_back_from_loading();
        self.imp()
            .all_images_stack
            .set_visible_child_name("all_images");
        if self.imp().leaf.visible_child_name().unwrap() == "main" {
            self.switch_to_stack_convert();
        }
    }

    fn selected_output(&self) -> Option<FileType> {
        match self.imp().output_filetype.selected_item() {
            Some(o) => match o.downcast::<gtk::StringObject>() {
                Ok(o) => Some(FileType::from_string(&o.string().as_str().to_lowercase()).unwrap()),
                Err(_) => None,
            },
            None => None,
        }
    }

    fn selected_compression(&self) -> Option<CompressionType> {
        match self.imp().output_compression.is_visible() {
            true => match self.imp().output_compression_value.is_active() {
                true => Some(CompressionType::Zip),
                false => Some(CompressionType::Directory),
            },
            false => None,
        }
    }

    fn files_count(&self) -> usize {
        (self.imp().input_file_store.n_items() as usize) - self.imp().removed.borrow().len()
    }

    fn load_pixbuf(&self) {
        let files = self.active_files();

        let file_path_things = files
            .iter()
            .map(|f| (f.kind().supports_pixbuf(), f.path()))
            .collect_vec();

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let file_paths_pixbuf = file_path_things
                .into_iter()
                .enumerate()
                .map(|(i, (b, path))| {
                    let sender = sender.clone();
                    async move {
                        spawn(async move {
                            sender
                                .send((
                                    i,
                                    match b {
                                        true => Some(Texture::for_pixbuf(
                                            &Pixbuf::from_file_at_scale(&path, 500, -1, true)
                                                .unwrap(),
                                        )),
                                        false => None,
                                    },
                                ))
                                .expect("Concurrency issues");
                        })
                        .await
                    }
                })
                .collect_vec();

            rt.block_on(join_all(file_paths_pixbuf));
        });

        let completed = std::sync::Arc::new(AtomicUsize::new(0));
        let total = self.files_count();

        receiver.attach(
            None,
            clone!(@weak self as this => @default-return Continue(false), move |(i, p)| {
                if let Some(p) = p {
                    this.imp().input_file_store.item(i as u32).and_downcast::<InputFile>().unwrap().set_pixbuf(p);
                }
                glib::MainContext::default().iteration(true);
                let c = completed.clone();
                let x = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if x + 1 == total {
                    this.load_frames();
                    Continue(false)
                } else {
                    Continue(true)
                }
            }),
        );
    }

    async fn convert_start(&self, save_format: OutputType, path: String) {
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

        let output_type = self.selected_output().unwrap();

        let files = self.active_files();

        let dir = create_temporary_dir().await.unwrap();

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

        let (sender, receiver) = MainContext::channel(glib::PRIORITY_DEFAULT);

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

        self.switch_to_stack_converting();
    }
}

pub trait FileOperations {
    fn add_dialog(&self);
    fn open_files(&self, files: Vec<Option<InputFile>>);
    fn save_error(&self, error: Option<&str>);
    fn save_files(&self);
    fn open_load(&self);
    fn open_error(&self, error: Option<&str>);
    fn add_success_wrapper(&self, files: Vec<InputFile>);
}

trait StackNavigation {
    fn switch_to_stack_convert(&self);
    fn switch_to_stack_converting(&self);
    fn switch_to_stack_welcome(&self);
    fn switch_to_main_leaf(&self);
    fn switch_to_stack_invalid_image(&self);
    fn switch_to_stack_loading(&self);
    fn switch_back_from_loading(&self);
    fn switch_to_stack_loading_generally(&self);
}

pub trait WindowUI {
    fn update_options(&self);
    fn update_output_options(&self);
    fn update_compression_options(&self);
    fn update_advanced_options(&self);
    fn update_width_from_height(&self);
    fn update_height_from_width(&self);
    fn update_resize(&self);
    fn update_full_image_container(&self);
    fn update_image_container(&self, count: usize, remaining_visible: bool);
}

trait ConvertArguments {
    fn get_quality_argument(&self) -> usize;
    fn get_dpi_argument(&self) -> usize;
    fn get_bgcolor_argument(&self) -> Color;
    fn get_filter_argument(&self) -> Option<ResizeFilter>;
    fn get_resize_argument(&self) -> ResizeArgument;
}
trait ConvertOperations {
    fn convert_start_wrapper(&self, save_format: OutputType, path: String);
    fn move_output(
        &self,
        save_format: OutputType,
        path: String,
        output_files: Vec<String>,
        dir_path: String,
    );
    fn convert_failed(&self, error_message: String, temp_dir_path: String);
    fn convert_success(&self, temp_dir_path: String, path: String, save_format: OutputType);
    fn convert_clean(&self, temp_dir_path: String);
    fn convert_cancel(&self);
}

trait SettingsStore {
    fn save_window_size(&self) -> Result<(), glib::BoolError>;
    fn load_window_size(&self);
    fn save_options(&self) -> Result<(), glib::BoolError>;
    fn load_options(&self);
    fn save_selected_output(&self) -> Result<(), glib::BoolError>;
    fn load_selected_output(&self) -> FileType;
    fn save_selected_compression(&self) -> Result<(), glib::BoolError>;
    fn load_selected_compression(&self) -> CompressionType;
}

impl ConvertOperations for AppWindow {
    fn convert_start_wrapper(&self, save_format: OutputType, path: String) {
        MainContext::default().spawn_local(clone!(@weak self as this => async move {
            this.convert_start(save_format, path).await;
        }));
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

                let (sender, receiver) = MainContext::channel(glib::PRIORITY_DEFAULT);

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
                let (sender, receiver) = MainContext::channel(glib::PRIORITY_DEFAULT);

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
                let (sender, receiver) = MainContext::channel(glib::PRIORITY_DEFAULT);

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
        self.convert_clean(temp_dir_path);
        if self
            .imp()
            .is_canceled
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        let mut current_jobs = self.imp().current_jobs.borrow_mut();
        self.imp()
            .is_canceled
            .store(true, std::sync::atomic::Ordering::SeqCst);
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

        self.switch_to_stack_convert();
    }

    fn convert_success(&self, temp_dir_path: String, path: String, save_format: OutputType) {
        self.convert_clean(temp_dir_path);
        self.imp()
            .is_canceled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let toast = adw::Toast::new(&gettext("Image converted"));
        toast.set_button_label(Some(&gettext("Open")));
        toast.connect_button_clicked(clone!(@weak self as this => move |_| {
            let p = path.clone();
            MainContext::default().spawn_local(clone!(@weak this as other_this => async move {
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
        self.switch_to_stack_convert();
    }

    fn convert_clean(&self, temp_dir_path: String) {
        clean_dir(temp_dir_path);
        self.imp().convert_button.set_sensitive(true);
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
                    this.switch_to_stack_convert();
                    this.show_toast(&gettext("Converting Cancelled"));
                }
            }),
        );

        stop_converting_dialog.present();
    }
}

impl ConvertArguments for AppWindow {
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
        match self.imp().resize_filter_default.is_active() {
            true => Some(ResizeFilter::Default),
            false => Some(ResizeFilter::Point),
        }
    }

    fn get_resize_argument(&self) -> ResizeArgument {
        let imp = self.imp();

        let resize_type = ResizeType::from_index(imp.resize_type.selected() as usize).unwrap();

        match resize_type {
            ResizeType::Percentage => ResizeArgument::Percentage {
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
            },
            ResizeType::ExactPixels => ResizeArgument::ExactPixels {
                width: imp.resize_width_value.text().to_string().parse().unwrap(),
                height: imp.resize_height_value.text().to_string().parse().unwrap(),
            },
        }
    }
}

impl WindowUI for AppWindow {
    fn update_options(&self) {
        let imp = self.imp();
        imp.resize_scale_height_value.set_text("100");
        imp.resize_scale_width_value.set_text("100");
        if let (Some(image_width), Some(image_height)) =
            (imp.image_width.get(), imp.image_height.get())
        {
            imp.resize_width_value.set_text(&image_width.to_string());
            imp.resize_height_value.set_text(&image_height.to_string());
        } else {
            imp.resize_width_value.set_text("");
            imp.resize_height_value.set_text("");
        }
        self.update_output_options();
        self.update_advanced_options();
    }

    fn update_output_options(&self) {
        let previous_option = self
            .selected_output()
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

    fn update_compression_options(&self) {
        let files = self.active_files();
        let multiple_files = files.len() > 1;
        let multiple_frames = multiple_files || files.iter().map(|i| i.frames()).sum::<usize>() > 1;
        let output_option = self.selected_output().unwrap();
        match (multiple_files, multiple_frames) {
            (false, false) => {
                self.imp().output_compression.set_visible(false);
            }
            (false, true) if output_option.supports_animation() => {
                self.imp().output_compression.set_visible(false);
            }
            _ => {
                let previous_option = self
                    .selected_compression()
                    .unwrap_or(self.load_selected_compression());

                self.imp().output_compression.set_visible(true);

                match previous_option {
                    CompressionType::Zip => self.imp().output_compression_value.set_active(true),
                    _ => self.imp().output_compression_value.set_active(false),
                }
            }
        }
    }

    fn update_advanced_options(&self) {
        let imp = self.imp();

        let input_files = self.active_files();
        let input_filetypes: Vec<FileType> = input_files.iter().map(|inf| inf.kind()).collect();
        let Some(output_filetype) = FileType::output_formats(self.imp().settings.boolean("show-less-popular")).nth(imp.output_filetype.selected() as usize) else {
            return;
        };

        imp.quality_row.set_visible(false);
        imp.bgcolor_row.set_visible(false);
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
                imp.bgcolor.set_rgba(
                    &gdk::RGBA::builder()
                        .red(0.00)
                        .green(0.0)
                        .blue(0.0)
                        .alpha(0.0000001)
                        .build(),
                );
                let color_dialog = imp.bgcolor.dialog().unwrap();
                color_dialog.set_with_alpha(true);
            } else {
                imp.bgcolor.set_rgba(&gdk::RGBA::WHITE);
                let color_dialog = imp.bgcolor.dialog().unwrap();
                color_dialog.set_with_alpha(false);
            }
        }

        if input_filetypes
            .iter()
            .all(|input_filetype| *input_filetype == FileType::Svg)
        {
            imp.resize_filter_row.set_visible(false);
        } else {
            imp.resize_filter_row.set_visible(true);
        }

        if input_filetypes
            .iter()
            .any(|input_filetype| *input_filetype == FileType::Pdf)
        {
            imp.dpi_row.set_visible(true);
        }
    }

    fn update_width_from_height(&self) {
        if self.imp().link_axis.is_active() && self.imp().link_axis.is_visible() {
            if let (Some(image_width), Some(image_height)) =
                (self.imp().image_width.get(), self.imp().image_height.get())
            {
                let old_value = self.imp().resize_width_value.text().as_str().to_owned();
                let other_text = self.imp().resize_height_value.text().as_str().to_owned();
                if other_text.is_empty() {
                    return;
                }

                let other_way = generate_height_from_width(
                    old_value.parse().unwrap_or(0),
                    (image_width, image_height),
                )
                .to_string();

                if other_way == other_text {
                    return;
                }

                let new_value = generate_width_from_height(
                    other_text.parse().unwrap_or(0),
                    (image_width, image_height),
                )
                .to_string();

                if old_value != new_value && new_value != "0" {
                    self.imp().resize_width_value.set_text(&new_value);
                }
            }
        }
    }

    fn update_height_from_width(&self) {
        if self.imp().link_axis.is_active() && self.imp().link_axis.is_visible() {
            if let (Some(image_width), Some(image_height)) =
                (self.imp().image_width.get(), self.imp().image_height.get())
            {
                let old_value = self.imp().resize_height_value.text().as_str().to_owned();
                let other_text = self.imp().resize_width_value.text().as_str().to_owned();
                if other_text.is_empty() {
                    return;
                }

                let other_way = generate_width_from_height(
                    old_value.parse().unwrap_or(0),
                    (image_width, image_height),
                )
                .to_string();

                if other_way == other_text {
                    return;
                }

                let new_value = generate_height_from_width(
                    other_text.parse().unwrap_or(0),
                    (image_width, image_height),
                )
                .to_string();

                if old_value != new_value && new_value != "0" {
                    self.imp().resize_height_value.set_text(&new_value);
                }
            }
        }
    }

    fn update_resize(&self) {
        let imp = self.imp();

        let resize_type = ResizeType::from_index(imp.resize_type.selected() as usize).unwrap();
        imp.resize_height_value.set_visible(false);
        imp.resize_width_value.set_visible(false);
        imp.resize_scale_height_value.set_visible(false);
        imp.resize_scale_width_value.set_visible(false);
        imp.link_axis.set_visible(false);

        match resize_type {
            ResizeType::Percentage => {
                imp.resize_scale_width_value.set_visible(true);
                imp.resize_scale_height_value.set_visible(true);
                imp.link_axis.set_visible(true);
            }
            ResizeType::ExactPixels => {
                imp.resize_width_value.set_visible(true);
                imp.resize_height_value.set_visible(true);
                if self.imp().image_width.get().is_some() && self.imp().image_height.get().is_some()
                {
                    imp.link_axis.set_visible(true);
                }
            }
        }
    }

    fn update_full_image_container(&self) {
        let imp = self.imp();

        let input_files = self
            .active_files()
            .into_iter()
            .map(|f| {
                let (k, d) = (f.kind(), f.dimensions());
                (f, k, d)
            })
            .collect_vec();

        while let Some(child) = imp.full_image_container.first_child() {
            imp.full_image_container.remove(&child);
        }

        for (i, (f, file_type, dims)) in input_files.into_iter().enumerate() {
            let caption = match dims {
                Some((w, h)) => {
                    format!("{} · {}×{}", file_type.as_display_string(), w, h,)
                }
                None => file_type.as_display_string().to_owned(),
            };

            let (w, h) = dims.unwrap_or((0, 0));

            let image_thumbnail =
                ImageThumbnail::new(f.pixbuf().as_ref(), &caption, w as u32, h as u32);

            let image_flow_box_child = gtk::FlowBoxChild::new();
            image_flow_box_child.set_child(Some(&image_thumbnail));

            image_flow_box_child.update_property(&[Property::Label(&caption)]);

            imp.full_image_container.append(&image_flow_box_child);
            image_thumbnail.connect_remove_clicked(clone!(@weak self as this => move |_| {
                this.remove_file(i as u32);
                this.imp().image_container.invalidate_filter();
                this.imp().full_image_container.invalidate_filter();
            }));
        }
    }

    fn update_image_container(&self, count: usize, remaining_visible: bool) {
        let imp = self.imp();

        let input_files = self
            .active_files()
            .into_iter()
            .map(|f| {
                let (k, d) = (f.kind(), f.dimensions());
                (f, k, d)
            })
            .collect_vec();

        while let Some(child) = imp.image_container.first_child() {
            imp.image_container.remove(&child);
        }

        let removed = self.imp().removed.borrow().clone();

        for (i, (f, file_type, dims)) in input_files.into_iter().take(count).enumerate() {
            match removed.contains(&(i as u32)) {
                false => {
                    let caption = match dims {
                        Some((w, h)) => {
                            format!("{} · {}×{}", file_type.as_display_string(), w, h,)
                        }
                        None => file_type.as_display_string().to_owned(),
                    };

                    let (w, h) = dims.unwrap_or((0, 0));

                    let image_thumbnail =
                        ImageThumbnail::new(f.pixbuf().as_ref(), &caption, w as u32, h as u32);

                    let image_flow_box_child = gtk::FlowBoxChild::new();
                    image_flow_box_child.set_child(Some(&image_thumbnail));

                    image_flow_box_child.update_property(&[Property::Label(&caption)]);

                    imp.image_container.append(&image_flow_box_child);
                    image_thumbnail.connect_remove_clicked(clone!(@weak self as this => move |_| {
                        this.remove_file(i as u32);
                        this.imp().image_container.invalidate_filter();
                        this.imp().full_image_container.invalidate_filter();
                    }));
                }
                true => {
                    imp.image_container.append(&gtk::FlowBoxChild::new());
                }
            }
        }

        imp.elements.replace(count);

        if remaining_visible {
            let image_rest = ImageRest::new(self.files_count() - 5);
            let image_flow_box_child = gtk::FlowBoxChild::new();
            image_flow_box_child.set_child(Some(&image_rest));
            image_flow_box_child.set_focusable(false);
            imp.image_container.append(&image_flow_box_child);
            image_rest.connect_clicked(clone!(@weak self as this => move |_| {
                this.imp().leaf.navigate(adw::NavigationDirection::Forward);
            }));
        }

        match self.files_count() {
            1 => {
                imp.image_container.set_hexpand(true);
                imp.image_container.set_max_children_per_line(1);
                imp.image_container.set_halign(gtk::Align::Fill);
            }
            2 => {
                imp.image_container.set_hexpand(true);
                imp.image_container.set_max_children_per_line(2);
                imp.image_container.set_halign(gtk::Align::Fill);
            }
            _ => {
                imp.image_container.set_hexpand(false);
                imp.image_container.set_max_children_per_line(3);
                imp.image_container.set_halign(gtk::Align::Baseline);
            }
        }

        imp.image_container.invalidate_filter();
    }
}

impl StackNavigation for AppWindow {
    fn switch_to_stack_convert(&self) {
        self.switch_to_main_leaf();
        self.imp().add_button.set_visible(true);
        self.imp().stack.set_visible_child_name("stack_convert");
    }

    fn switch_to_stack_converting(&self) {
        self.imp().add_button.set_visible(false);
        self.imp().stack.set_visible_child_name("stack_converting");
    }

    fn switch_to_stack_welcome(&self) {
        self.switch_to_main_leaf();
        self.imp().add_button.set_visible(false);
        self.imp()
            .stack
            .set_visible_child_name("stack_welcome_page");
    }

    fn switch_to_main_leaf(&self) {
        self.set_title(Some(&gettext("Converter")));
        self.imp().leaf.set_visible_child_name("main");
    }

    fn switch_to_stack_invalid_image(&self) {
        self.switch_to_main_leaf();
        self.imp().add_button.set_visible(false);
        self.imp()
            .stack
            .set_visible_child_name("stack_invalid_image");
    }

    fn switch_to_stack_loading(&self) {
        self.switch_to_main_leaf();
        self.imp().add_button.set_visible(false);
        self.imp().stack.set_visible_child_name("stack_loading");
        self.imp().loading_spinner.start();
    }

    fn switch_back_from_loading(&self) {
        self.imp().loading_spinner.stop();
        self.imp().loading_spinner_images.stop();
        self.imp().other_add_button.set_visible(true);
        self.imp().back_button.set_visible(true);
    }

    fn switch_to_stack_loading_generally(&self) {
        if self.imp().leaf.visible_child_name().unwrap() == "main" {
            self.switch_to_stack_loading();
        } else {
            self.imp().other_add_button.set_visible(false);
            self.imp().back_button.set_visible(false);
            self.imp()
                .all_images_stack
                .set_visible_child_name("stack_loading");
            self.imp().loading_spinner_images.start();
        }
    }
}

impl SettingsStore for AppWindow {
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

        let output_format = self.selected_output().unwrap();

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

        if let Some(output_format) = self.selected_compression() {
            let pos = CompressionType::possible_output(false)
                .position(|&x| x == output_format)
                .unwrap();

            imp.settings.set_enum("compression-format", pos as i32)?;
        }
        Ok(())
    }

    fn load_selected_compression(&self) -> CompressionType {
        let imp = self.imp();

        **CompressionType::possible_output(false)
            .collect_vec()
            .get(imp.settings.enum_("compression-format") as usize)
            .unwrap_or(&&CompressionType::Directory)
    }
}

impl FileOperations for AppWindow {
    fn open_files(&self, files: Vec<Option<InputFile>>) {
        let files = files.into_iter().flatten().collect_vec();
        self.add_success_wrapper(files);
    }

    fn save_error(&self, error: Option<&str>) {
        if let Some(s) = error {
            self.show_toast(s);
        }
    }

    fn save_files(&self) {
        let files = self.active_files();
        let multiple_files = files.len() > 1;
        let multiple_frames = multiple_files || files.iter().map(|i| i.frames()).sum::<usize>() > 1;
        let output_option = self.selected_output().unwrap();
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
                OutputType::Compression(self.selected_compression().unwrap()),
                "images".to_owned(),
            ),
        };

        let sandboxed = files.iter().any(|f| f.is_behind_sandbox());

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
                AppWindow::convert_start_wrapper,
                AppWindow::save_error,
            );
        } else {
            FileChooser::choose_output_folder_wrapper(
                self,
                default_folder,
                AppWindow::convert_start_wrapper,
                AppWindow::save_error,
            );
        }
    }

    fn add_dialog(&self) {
        FileChooser::open_files_wrapper(
            self,
            vec![],
            AppWindow::open_load,
            AppWindow::add_success_wrapper,
            AppWindow::open_error,
        );
    }

    fn open_error(&self, error: Option<&str>) {
        if error.is_some() {
            self.switch_to_stack_invalid_image();
        }
    }

    fn open_load(&self) {
        self.switch_to_stack_loading_generally();
        self.imp().loading_spinner.start();
    }

    fn add_success_wrapper(&self, files: Vec<InputFile>) {
        MainContext::default().spawn_local(clone!(@weak self as this => async move {
            this.open_success(files).await;
        }));
    }
}

fn generate_width_from_height(height: u32, image_dim: (u32, u32)) -> u32 {
    ((height as f64) * (image_dim.0 as f64) / (image_dim.1 as f64)).round() as u32
}

fn generate_height_from_width(width: u32, image_dim: (u32, u32)) -> u32 {
    ((width as f64) * (image_dim.1 as f64) / (image_dim.0 as f64)).round() as u32
}
