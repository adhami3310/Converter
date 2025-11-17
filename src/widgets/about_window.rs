use adw::prelude::*;
use gettextrs::gettext;
use glib::object::IsA;
use gtk::License;

//code 'inspired' by https://gitlab.com/news-flash/news_flash_gtk/-/blob/master/src/about_dialog.rs

//this is non-translatable information, so it can be const
pub const DEVELOPERS: &[&str] = &["Khaleel Al-Adhami <khaleel.aladhami@gmail.com>"];

#[derive(Clone, Debug)]
pub struct SwitcherooAbout;

impl SwitcherooAbout {
    pub fn show<W: IsA<gtk::Widget>>(window: &W) {
        let about = adw::AboutDialog::from_appdata(
            "/io/gitlab/adhami3310/Converter/io.gitlab.adhami3310.Converter.metainfo.xml",
            Some(crate::config::VERSION),
        );
        about.set_developers(DEVELOPERS);
        about.set_translator_credits(&gettext("translator-credits"));

        about.add_other_app(
            "io.gitlab.adhami3310.Impression",
            // Translators: Metainfo for the app Impression. <https://gitlab.com/adhami3310/Impression>
            &gettext("Impression"),
            // Translators: Metainfo for the app Impression. <https://gitlab.com/adhami3310/Impression>
            &gettext("Create bootable drives"),
        );
        about.add_other_app(
            "io.gitlab.adhami3310.Footage",
            // Translators: Metainfo for the app Footage. <https://gitlab.com/adhami3310/Footage>
            &gettext("Footage"),
            // Translators: Metainfo for the app Footage. <https://gitlab.com/adhami3310/Footage>
            &gettext("Polish your videos"),
        );
        about.add_acknowledgement_section(
            Some(&gettext("Code and Design Borrowed from")),
            &[
                "GTK-Rust-Template https://gitlab.gnome.org/World/Rust/gtk-rust-template",
                "Amberol https://gitlab.gnome.org/World/amberol",
                "Upscaler https://gitlab.gnome.org/World/Upscaler",
                "Avvie https://github.com/Taiko2k/Avvie",
                "Bottles https://github.com/bottlesdevs/Bottles",
                "Loupe https://gitlab.gnome.org/GNOME/loupe",
                "Totem https://gitlab.gnome.org/GNOME/totem",
            ],
        );
        about.add_acknowledgement_section(
            Some(&gettext("Sample Image from")),
            &["Samuel Custodio https://github.com/samuelcust/flappy-bird-assets"],
        );
        about.add_legal_section("ImageMagick", None, License::MitX11, None);
        about.present(Some(window));
    }
}
