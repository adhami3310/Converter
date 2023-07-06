use std::slice::Iter;

#[derive(Clone, Copy, Debug, glib::Enum, PartialEq, Default, Eq, Hash)]
#[enum_type(name = "SwitcherooFiletype")]
pub enum FileType {
    #[enum_value(name = "PNG")]
    Png,
    #[enum_value(name = "JPEG")]
    Jpeg,
    #[enum_value(name = "JPG")]
    Jpg,
    #[enum_value(name = "WEBP")]
    Webp,
    #[enum_value(name = "SVG")]
    Svg,
    #[enum_value(name = "HEIF")]
    Heif,
    #[enum_value(name = "HEIC")]
    Heic,
    #[enum_value(name = "BMP")]
    Bmp,
    #[enum_value(name = "AVIF")]
    Avif,
    #[enum_value(name = "JXL")]
    Jxl,
    #[enum_value(name = "TIFF")]
    Tiff,
    #[enum_value(name = "PDF")]
    Pdf,
    #[enum_value(name = "GIF")]
    Gif,
    #[enum_value(name = "ICO")]
    Ico,
    #[enum_value(name = "DDS")]
    Dds,
    #[enum_value(name = "Unknown")]
    #[default]
    Unknown,
}

use FileType::*;

impl FileType {
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Png | Jpg
                | Webp
                | Svg
                | Heif
                | Heic
                | Bmp
                | Avif
                | Jxl
                | Tiff
                | Pdf
                | Gif
                | Ico
                | Jpeg
                | Dds
        )
    }

    pub fn supports_animation(&self) -> bool {
        matches!(self, Webp | Gif)
    }

    pub fn is_lossy(&self) -> bool {
        matches!(
            self,
            Jpg | Jpeg | Webp | Heif | Heic | Avif | Jxl | Tiff | Pdf | Dds
        )
    }

    pub fn supports_alpha(&self) -> bool {
        matches!(
            self,
            Png | Webp | Svg | Heif | Heic | Avif | Jxl | Pdf | Ico | Gif
        )
    }

    pub fn supports_pixbuf(&self) -> bool {
        !matches!(self, Pdf | Dds | Ico)
    }

    pub fn is_output(&self) -> bool {
        matches!(
            self,
            Png | Jpg | Jpeg | Webp | Heif | Heic | Bmp | Avif | Jxl | Tiff | Pdf | Gif | Ico | Dds
        )
    }

    pub fn iterator() -> Iter<'static, Self> {
        static FILETYPES: [FileType; 15] = [
            Png, Jpg, Jpeg, Webp, Svg, Heif, Heic, Bmp, Avif, Jxl, Tiff, Pdf, Gif, Ico, Dds,
        ];
        FILETYPES.iter()
    }

    pub fn input_formats() -> Iter<'static, Self> {
        static FILETYPES: [FileType; 15] = [
            Png, Jpg, Jpeg, Webp, Svg, Heif, Heic, Bmp, Avif, Jxl, Tiff, Pdf, Gif, Ico, Dds,
        ];
        FILETYPES.iter()
    }

    pub fn output_formats(hidden: bool) -> Iter<'static, Self> {
        static ALL_FILETYPES: [FileType; 13] = [
            Png, Jpg, Webp, Heif, Heic, Bmp, Avif, Jxl, Tiff, Pdf, Gif, Ico, Dds,
        ];
        static POPULAR_FILETYPES: [FileType; 7] = [Png, Jpg, Webp, Heif, Pdf, Gif, Ico];
        match hidden {
            true => ALL_FILETYPES.iter(),
            false => POPULAR_FILETYPES.iter(),
        }
    }

    pub fn as_mime(&self) -> &'static str {
        match self {
            Png => "image/png",
            Jpg => "image/jpeg",
            Jpeg => "image/jpeg",
            Webp => "image/webp",
            Svg => "image/svg+xml",
            Heif => "image/heif",
            Heic => "image/heic",
            Bmp => "image/bmp",
            Avif => "image/avif",
            Jxl => "image/jxl",
            Tiff => "image/tiff",
            Pdf => "application/pdf",
            Gif => "image/gif",
            Ico => "image/x-icon",
            Dds => "image/vnd-ms.dds",
            // ZIP => "application/zip",
            // TAR => "application/gzip",
            Unknown => "",
        }
    }

    pub fn from_mimetype(mimetype: &str) -> Option<Self> {
        match mimetype {
            "image/png" => Some(Png),
            "image/jpeg" => Some(Jpg),
            "image/jpg" => Some(Jpg),
            "image/webp" => Some(Webp),
            "image/svg+xml" => Some(Svg),
            "image/heif" => Some(Heif),
            "image/heic" => Some(Heic),
            "image/bmp" => Some(Bmp),
            "image/avif" => Some(Avif),
            "image/jxl" => Some(Jxl),
            "image/tiff" => Some(Tiff),
            "application/pdf" => Some(Pdf),
            "image/gif" => Some(Gif),
            "image/x-icon" => Some(Ico),
            "image/vnd-ms.dds" => Some(Dds),
            _ => None
        }
    }

    pub fn as_extension(&self) -> &str {
        match self {
            Png => "png",
            Jpg => "jpg",
            Jpeg => "jpeg",
            Webp => "webp",
            Svg => "svg",
            Heif => "heif",
            Heic => "heic",
            Bmp => "bmp",
            Avif => "avif",
            Jxl => "jxl",
            Tiff => "tiff",
            Pdf => "pdf",
            Gif => "gif",
            Ico => "ico",
            Dds => "dds",
            // ZIP => "zip",
            // TAR => "tar.gz",
            Unknown => "",
        }
    }

    pub fn as_display_string(&self) -> String {
        self.as_extension().to_uppercase()
    }

    pub fn from_string(extension: &str) -> Option<Self> {
        match extension {
            "png" => Some(Png),
            "jpg" => Some(Jpg),
            "jpeg" => Some(Jpg),
            "webp" => Some(Webp),
            "svg" => Some(Svg),
            "heif" => Some(Heif),
            "heic" => Some(Heic),
            "bmp" => Some(Bmp),
            "avif" => Some(Avif),
            "jxl" => Some(Jxl),
            "tiff" => Some(Tiff),
            "pdf" => Some(Pdf),
            "gif" => Some(Gif),
            "ico" => Some(Ico),
            "dds" => Some(Dds),
            // "zip" => Some(ZIP),
            // "tar.gz" => Some(TAR),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, glib::Enum, PartialEq, Eq, Hash)]
#[enum_type(name = "SwitcherooCompressionType")]
pub enum CompressionType {
    #[enum_value(name = "ZIP")]
    Zip,
    #[enum_value(name = "Directory")]
    Directory,
}

use CompressionType::*;

impl CompressionType {
    pub fn is_compression(&self) -> bool {
        matches!(self, Zip)
    }

    pub fn iterator() -> Iter<'static, Self> {
        static COMPRESSION_TYPES: [CompressionType; 2] = [Zip, Directory];
        COMPRESSION_TYPES.iter()
    }

    pub fn compression_formats() -> Iter<'static, Self> {
        static COMPRESSION_TYPES: [CompressionType; 1] = [Zip];
        COMPRESSION_TYPES.iter()
    }

    pub fn possible_output(sandboxed: bool) -> Iter<'static, Self> {
        static COMPRESSION_TYPES: [CompressionType; 1] = [Zip];
        static ALL_TYPES: [CompressionType; 2] = [Zip, Directory];
        match sandboxed {
            true => COMPRESSION_TYPES.iter(),
            false => ALL_TYPES.iter(),
        }
    }

    pub fn as_mime(&self) -> &'static str {
        match self {
            Zip => "application/zip",
            Directory => "inode/directory",
        }
    }

    pub fn as_extension(&self) -> &str {
        match self {
            Zip => "zip",
            Directory => "directory",
        }
    }

    pub fn as_display_string(&self) -> String {
        match self {
            Directory => "Directory".to_owned(),
            x => x.as_extension().to_uppercase(),
        }
    }

    pub fn from_string(extension: &str) -> Option<Self> {
        match extension {
            "zip" => Some(Zip),
            "directory" => Some(Directory),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputType {
    File(FileType),
    Compression(CompressionType),
}

impl OutputType {
    pub fn as_mime(&self) -> &'static str {
        match self {
            OutputType::File(f) => f.as_mime(),
            OutputType::Compression(f) => f.as_mime(),
        }
    }

    pub fn from_string(extension: &str) -> Option<Self> {
        match extension {
            "zip" => Some(OutputType::Compression(Zip)),
            "directory" => Some(OutputType::Compression(Directory)),
            "png" => Some(OutputType::File(Png)),
            "jpg" => Some(OutputType::File(Jpg)),
            "jpeg" => Some(OutputType::File(Jpg)),
            "webp" => Some(OutputType::File(Webp)),
            "svg" => Some(OutputType::File(Svg)),
            "heif" => Some(OutputType::File(Heif)),
            "heic" => Some(OutputType::File(Heic)),
            "bmp" => Some(OutputType::File(Bmp)),
            "avif" => Some(OutputType::File(Avif)),
            "jxl" => Some(OutputType::File(Jxl)),
            "tiff" => Some(OutputType::File(Tiff)),
            "pdf" => Some(OutputType::File(Pdf)),
            "gif" => Some(OutputType::File(Gif)),
            "ico" => Some(OutputType::File(Ico)),
            "dds" => Some(OutputType::File(Dds)),
            _ => None,
        }
    }

    pub fn as_extension(&self) -> &str {
        match self {
            OutputType::File(f) => f.as_extension(),
            OutputType::Compression(f) => f.as_extension(),
        }
    }

    // pub fn as_display_string(&self) -> String {
    //     match self {
    //         OutputType::Compression(x) => x.as_display_string(),
    //         x => x.as_extension().to_uppercase(),
    //     }
    // }
}
