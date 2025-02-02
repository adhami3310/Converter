use crate::config::{APP_ID, VERSION};
use adw::prelude::*;
use gettextrs::gettext;
use glib::object::IsA;
use gtk::License;

//code 'inspired' by https://gitlab.com/news-flash/news_flash_gtk/-/blob/master/src/about_dialog.rs

//this is non-translatable information, so it can be const
pub const WEBSITE: &str = "https://gitlab.com/adhami3310/Switcheroo";
pub const ISSUE_TRACKER: &str = "https://gitlab.com/adhami3310/Switcheroo/-/issues";
pub const DEVELOPERS: &[&str] = &["Khaleel Al-Adhami <khaleel.aladhami@gmail.com>"];

#[derive(Clone, Debug)]
pub struct SwitcherooAbout;

impl SwitcherooAbout {
    pub fn show<W: IsA<gtk::Widget>>(window: &W) {
        let about_window = adw::AboutDialog::builder()
            .application_icon(APP_ID)
            .application_name(gettext("Switcheroo"))
            .developer_name("Khaleel Al-Adhami")
            .developers(DEVELOPERS)
            // Translators: Replace "translator-credits" with your names, one name per line
            .translator_credits(gettext("translator-credits"))
            .license_type(License::Gpl30Only)
            .version(VERSION)
            .website(WEBSITE)
            .issue_url(ISSUE_TRACKER)
            .build();
        about_window.add_acknowledgement_section(
            Some(&gettext("Code and Design Borrowed from")),
            &[
                "GTK-Rust-Template https://gitlab.gnome.org/World/Rust/gtk-rust-template",
                "Amberol https://gitlab.gnome.org/GNOME/gimp",
                "Upscaler https://gitlab.com/TheEvilSkeleton/Upscaler",
                "Avvie https://github.com/Taiko2k/Avvie",
                "Bottles https://github.com/bottlesdevs/Bottles",
                "Loupe https://gitlab.gnome.org/BrainBlasted/loupe",
                "Totem https://gitlab.gnome.org/GNOME/totem",
            ],
        );
        about_window.add_acknowledgement_section(
            Some(&gettext("Sample Image from")),
            &["Samuel Custodio https://github.com/samuelcust/flappy-bird-assets"],
        );
        about_window.add_legal_section("ImageMagick", None, License::MitX11, None);
        about_window.present(Some(window));
    }
}
