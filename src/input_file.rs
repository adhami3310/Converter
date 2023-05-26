use glib::{ParamSpec, ParamSpecEnum, ParamSpecString, Value};
use gtk::{
    gdk::gdk_pixbuf::{Colorspace, Pixbuf},
    gio, glib,
    prelude::*,
    subclass::prelude::*, cairo,
};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};

use crate::filetypes::FileType;

mod imp {
    use glib::{ParamSpecBoolean, ParamSpecObject};

    use super::*;

    #[derive(Debug)]
    pub struct InputFile {
        pub path: RefCell<String>,
        pub kind: Cell<FileType>,
        pub pixbuff: RefCell<Pixbuf>,
        pub frames: Cell<usize>,
        pub is_behind_sandbox: Cell<bool>,
        pub width: Cell<Option<usize>>,
        pub height: Cell<Option<usize>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InputFile {
        const NAME: &'static str = "ConverterInputFile";
        type Type = crate::input_file::InputFile;

        fn new() -> Self {
            Self {
                path: RefCell::new("/invalid-path".to_string()),
                kind: Cell::new(FileType::Unknown),
                pixbuff: RefCell::new(Pixbuf::new(Colorspace::Rgb, true, 8, 1, 1).unwrap()),
                frames: Cell::new(1),
                is_behind_sandbox: Cell::new(true),
                width: Cell::new(None),
                height: Cell::new(None),
            }
        }
    }

    impl ObjectImpl for InputFile {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("path").readwrite().build(),
                    ParamSpecEnum::builder::<FileType>("kind")
                        .readwrite()
                        .build(),
                    ParamSpecObject::builder::<Pixbuf>("pixbuff")
                        .readwrite()
                        .build(),
                    ParamSpecBoolean::builder("is-behind-sandbox")
                        .readwrite()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "path" => {
                    let p = value.get::<String>().expect("Value must be a String");
                    self.path.replace(p);
                }
                "kind" => {
                    let p = value.get::<FileType>().expect("Value must be a filetype");
                    self.kind.set(p);
                }
                "pixbuff" => {
                    let p = value.get::<Pixbuf>().expect("Value must be a Pixbuff");
                    self.pixbuff.replace(p);
                }
                "is-behind-sandbox" => {
                    let p = value.get::<bool>().expect("Value must be a boolean");
                    self.is_behind_sandbox.replace(p);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "path" => self.path.borrow().to_value(),
                "pixbuff" => self.pixbuff.borrow().to_value(),
                "kind" => self.kind.get().to_value(),
                "is-behind-sandbox" => self.is_behind_sandbox.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct InputFile(ObjectSubclass<imp::InputFile>);
}

impl Default for InputFile {
    fn default() -> Self {
        Self::empty()
    }
}

impl InputFile {
    pub fn new(file: &gio::File) -> Option<Self> {
        let path = file.path().unwrap();
        let is_behind_sandbox = !path.starts_with("/home");
        let extension = match path.extension() {
            Some(extension) => match extension.to_str() {
                Some(extension) => FileType::from_string(extension),
                None => None,
            },
            None => None,
        };
        extension.map(|extension| {
            glib::Object::builder::<Self>()
                .property("path", path.to_str().unwrap())
                .property("kind", extension)
                .property("is-behind-sandbox", is_behind_sandbox)
                .build()
        })
    }

    pub fn empty() -> Self {
        glib::Object::new()
    }

    pub async fn generate_pixbuff(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.pixbuf().is_some() || !self.kind().supports_pixbuff() {
            return Ok(());
        }

        let stream = gio::File::for_path(self.path())
            .read_future(glib::PRIORITY_DEFAULT)
            .await?;

        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream_future(&stream).await?;

        let pixbuf = &get_reduced(&pixbuf);

        self.set_property("pixbuff", pixbuf);

        Ok(())
    }

    pub fn pixbuf(&self) -> Option<Pixbuf> {
        let pixbuf = self.imp().pixbuff.borrow().clone();
        if pixbuf.width() > 1 && pixbuf.height() > 1 {
            Some(pixbuf)
        } else {
            None
        }
    }

    pub fn frames(&self) -> usize {
        self.imp().frames.get()
    }

    pub fn width(&self) -> Option<usize> {
        self.imp().width.get()
    }

    pub fn height(&self) -> Option<usize> {
        self.imp().height.get()
    }

    pub fn dimensions(&self) -> Option<(usize, usize)> {
        let (w, h) = (self.width(), self.height());
        w.map(|w| {
            h.map(|h| (w, h))
        }).flatten()
    }

    pub fn set_frames(&self, f: usize) {
        self.imp().frames.replace(f);
    }
    
    pub fn set_width(&self, f: usize) {
        self.imp().width.replace(Some(f));
    }
    
    pub fn set_height(&self, f: usize) {
        self.imp().height.replace(Some(f));
    }

    pub fn set_pixbuf(&self, p: Pixbuf) {
        self.imp().pixbuff.replace(p);
    }

    pub fn path(&self) -> String {
        self.imp().path.borrow().to_string()
    }

    pub fn is_behind_sandbox(&self) -> bool {
        self.imp().is_behind_sandbox.get()
    }

    pub fn kind(&self) -> FileType {
        self.imp().kind.get()
    }
}


fn get_reduced(p: &Pixbuf) -> Pixbuf {
    let max_side = 200.0;
    let (width, height) = (p.width() as f64, p.height() as f64);
    let max_original_side = std::cmp::min(width as usize, height as usize) as f64;
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

// pub fn get_square(p: &Pixbuf) -> Pixbuf {
//     let side = std::cmp::min(p.width(), p.height());
//     let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, side, side).unwrap();
//     let context = cairo::Context::new(&surface).unwrap();
//     context.set_source_pixbuf(&p, ((p.width() - side) as f64) / -2.0, ((p.height() - side) as f64) / -2.0);
//     context.paint().unwrap();
//     gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, side, side).unwrap()
// }