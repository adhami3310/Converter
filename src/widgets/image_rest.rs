use glib::SignalHandlerId;
use gtk::{gio, glib, subclass::prelude::*, traits::ButtonExt};

mod imp {

    use super::*;

    use gtk::CompositeTemplate;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/gitlab/adhami3310/Converter/blueprints/image-rest.ui")]
    pub struct ImageRest {
        #[template_child]
        pub image: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageRest {
        const NAME: &'static str = "ImageRest";
        type Type = super::ImageRest;
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
            }
        }
    }

    impl ObjectImpl for ImageRest {}

    impl WidgetImpl for ImageRest {}

    impl FlowBoxChildImpl for ImageRest {}
}

glib::wrapper! {
    pub struct ImageRest(ObjectSubclass<imp::ImageRest>)
        @extends gtk::FlowBoxChild, gtk::Widget,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

#[gtk::template_callbacks]
impl ImageRest {
    pub fn new() -> Self {
        let bin = glib::Object::builder::<ImageRest>().build();

        bin.setup_callbacks();

        bin
    }

    fn setup_callbacks(&self) {
        //load imp
        // let imp = self.imp();
    }

    pub fn connect_clicked<F>(&self, func: F) -> SignalHandlerId
    where
        F: Fn(&gtk::Button) + 'static,
    {
        self.imp().image.connect_clicked(func)
    }
}
