use adw::prelude::*;
use glib::{SignalHandlerId, Value};
use gtk::{gdk::cairo, gdk::gdk_pixbuf::Pixbuf, gio, glib, subclass::prelude::*};
use once_cell::sync::Lazy;

fn get_reduced(p: &Pixbuf) -> Pixbuf {
    let max_side = 150.0;
    let (width, height) = (p.width() as f64, p.height() as f64);
    let max_original_side = std::cmp::max(width as usize, height as usize) as f64;
    let (scaled_width, scaled_height) = (
        width * max_side / max_original_side,
        height * max_side / max_original_side,
    );
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        scaled_width as i32,
        scaled_height as i32,
    )
    .unwrap();
    let context = cairo::Context::new(&surface).unwrap();
    context.scale(scaled_width / width, scaled_height / height);
    context.set_source_pixbuf(&p, 0.0, 0.0);
    context.paint().unwrap();
    context.scale(width / scaled_width, height / scaled_height);
    gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, scaled_width as i32, scaled_height as i32)
        .unwrap()
}

// fn get_square(p: &Pixbuf) -> Pixbuf {
//     let side = std::cmp::min(p.width(), p.height());
//     let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, side, side).unwrap();
//     let context = cairo::Context::new(&surface).unwrap();
//     context.set_source_pixbuf(&p, ((p.width() - side) as f64) / -2.0, 0.0);
//     context.paint().unwrap();
//     gdk::pixbuf_get_from_surface(&surface, 0, 0, side, side).unwrap()
// }

// fn crop_corners(p: &Pixbuf) -> Pixbuf {
//     let thumbnail_dimension = 512;
//     let radius = 25.0;
//     let thumbnail_dimension_f = thumbnail_dimension as f64;
//     let (width, height) = (p.width() as f64, p.height() as f64);
//     let surface = cairo::ImageSurface::create(
//         cairo::Format::ARgb32,
//         thumbnail_dimension,
//         thumbnail_dimension,
//     )
//     .unwrap();
//     let context = cairo::Context::new(&surface).unwrap();
//     let (fake_width, fake_height) = (1000.0, 1000.0);
//     context.new_path();
//     context.scale(
//         thumbnail_dimension_f / fake_width,
//         thumbnail_dimension_f / fake_height,
//     );
//     context.arc(radius, radius, radius, PI, PI * 1.5);
//     context.line_to(fake_width - radius, 0.0);
//     context.arc(fake_width - radius, radius, radius, PI * 1.5, PI * 2.0);
//     context.line_to(fake_width, fake_height - radius);
//     context.arc(
//         fake_width - radius,
//         fake_height - radius,
//         radius,
//         PI * 2.0,
//         PI * 2.5,
//     );
//     context.line_to(radius, fake_height);
//     context.arc(radius, fake_height - radius, radius, PI * 2.5, PI * 3.0);
//     context.line_to(0.0, radius);
//     context.clip();
//     context.scale(
//         fake_width / thumbnail_dimension_f,
//         fake_height / thumbnail_dimension_f,
//     );
//     context.scale(
//         thumbnail_dimension_f / width,
//         thumbnail_dimension_f / height,
//     );
//     context.set_source_pixbuf(&p, 0.0, 0.0);
//     context.paint().unwrap();
//     context.scale(
//         width / thumbnail_dimension_f,
//         height / thumbnail_dimension_f,
//     );
//     gdk::pixbuf_get_from_surface(&surface, 0, 0, 512, 512).unwrap()
// }

mod imp {

    use super::*;

    use glib::{ParamSpec, ParamSpecObject, ParamSpecString};
    use gtk::CompositeTemplate;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/gitlab/adhami3310/Converter/blueprints/image-thumbnail.ui")]
    pub struct ImageThumbnail {
        #[template_child]
        pub image: TemplateChild<gtk::Image>,
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub content: TemplateChild<gtk::Label>,
        #[template_child]
        pub remove: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageThumbnail {
        const NAME: &'static str = "ImageThumbnail";
        type Type = super::ImageThumbnail;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                image: TemplateChild::default(),
                picture: TemplateChild::default(),
                content: TemplateChild::default(),
                remove: TemplateChild::default(),
            }
        }
    }

    impl ObjectImpl for ImageThumbnail {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecObject::builder::<Pixbuf>("image")
                        .write_only()
                        .build(),
                    ParamSpecString::builder("content").write_only().build(),
                    ParamSpecObject::builder::<gtk::Button>("remove")
                        .read_only()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "image" => {
                    let p = value
                        .get::<Option<Pixbuf>>()
                        .expect("Value must be a Pixbuff");
                    match p {
                        Some(p) => {
                            self.picture.set_pixbuf(Some(&get_reduced(&p)));
                            self.image.set_visible(false);
                            self.picture.set_visible(true);
                        }
                        None => {
                            self.image.set_icon_name(Some("image-symbolic"));
                            self.image.set_visible(true);
                            self.picture.set_visible(false);
                        }
                    }
                }
                "content" => {
                    let p = value.get::<&str>().expect("Value must be a string");
                    self.content.set_text(p);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "remove" => self.remove.to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for ImageThumbnail {}

    impl FlowBoxChildImpl for ImageThumbnail {}
}

glib::wrapper! {
    pub struct ImageThumbnail(ObjectSubclass<imp::ImageThumbnail>)
        @extends gtk::FlowBoxChild, gtk::Widget,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

#[gtk::template_callbacks]
impl ImageThumbnail {
    pub fn new(image: Option<Pixbuf>, content: &str) -> Self {
        let bin = glib::Object::builder::<ImageThumbnail>()
            .property("image", image)
            .property("content", content)
            .build();

        bin.setup_callbacks();

        bin
    }

    fn setup_callbacks(&self) {
        //load imp
        // let imp = self.imp();
    }

    pub fn connect_remove_clicked<F>(&self, func: F) -> SignalHandlerId
    where
        F: Fn(&gtk::Button) + 'static,
    {
        self.imp().remove.connect_clicked(func)
    }
}
