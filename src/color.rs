#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn as_hex_string(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            self.red, self.green, self.blue, self.alpha
        )
    }
}

impl From<gtk::gdk::RGBA> for Color {
    fn from(color: gtk::gdk::RGBA) -> Self {
        Color::rgba(
            (255f32 * color.red()) as u8,
            (255f32 * color.green()) as u8,
            (255f32 * color.blue()) as u8,
            (255f32 * color.alpha()) as u8,
        )
    }
}

impl From<Color> for gtk::gdk::RGBA {
    fn from(color: Color) -> Self {
        gtk::gdk::RGBA::new(
            color.red as f32 / 255f32,
            color.green as f32 / 255f32,
            color.blue as f32 / 255f32,
            color.alpha as f32 / 255f32,
        )
    }
}
