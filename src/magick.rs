use crate::{color::Color, filetypes::FileType, window::ResizeFilter};
use gettextrs::gettext;
use glib::Bytes;
use gtk::gio::Cancellable;
use gtk::prelude::{FileExt, InputStreamExt};
use shared_child::SharedChild;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub async fn count_frames(path: String) -> Result<usize, ()> {
    let command = tokio::process::Command::new("magick")
        .arg("identify")
        .arg(path)
        .output()
        .await;

    match command {
        Ok(output) => match std::str::from_utf8(&output.stdout) {
            Ok(output_string) => Ok(output_string.lines().count()),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

pub async fn pixbuf_bytes(path: String) -> Bytes {
    let stream = gtk::gio::File::for_path(path)
        .read(Cancellable::NONE)
        .unwrap();


    stream.read_bytes(8192129, Cancellable::NONE).unwrap()
}

pub trait MagickArgument {
    fn get_argument(&self) -> Vec<String>;
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum SizeArgument {
//     Width(usize),
//     Height(usize),
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeArgument {
    Percentage { width: usize, height: usize },
    ExactPixels { width: usize, height: usize },
    // MinPixels {
    //     width: usize,
    //     height: usize,
    // },
    // MaxPixels {
    //     width: usize,
    //     height: usize,
    // },
    // Ratio {
    //     width: usize,
    //     height: usize,
    // },
}

// impl MagickArgument for SizeArgument {
//     fn get_argument(&self) -> Vec<String> {
//         match self {
//             SizeArgument::Width(w) => vec!["-size".to_owned(), w.to_string()],
//             SizeArgument::Height(h) => vec!["-size".to_owned(), format!("x{h}")],
//         }
//     }
// }

impl MagickArgument for ResizeFilter {
    fn get_argument(&self) -> Vec<String> {
        match self.as_display_string() {
            Some (x) => vec!["-filter".to_string(), x.to_owned()],
            None => vec![],
        }
    }
}

impl MagickArgument for ResizeArgument {
    fn get_argument(&self) -> Vec<String> {
        match self {
            ResizeArgument::Percentage { width, height } => {
                vec!["-resize".to_owned(), format!("{width}%x{height}%")]
            }
            ResizeArgument::ExactPixels {
                width,
                height,
            } => {
                vec!["-resize".to_owned(), format!("{width}x{height}!")]
            }
            // ResizeArgument::ExactPixels {
            //     width: Some(width),
            //     height: None,
            // } => {
            //     vec!["-resize".to_owned(), format!("{width}")]
            // }
            // ResizeArgument::ExactPixels {
            //     width: None,
            //     height: Some(height),
            // } => {
            //     vec!["-resize".to_owned(), format!("x{height}")]
            // }
            // ResizeArgument::MinPixels { width, height } => {
            //     vec!["-resize".to_owned(), format!("{width}x{height}")]
            // }
            // ResizeArgument::MaxPixels { width, height } => {
            //     vec!["-resize".to_owned(), format!("{width}x{height}^")]
            // }
            // ResizeArgument::Ratio { width, height } => {
            //     vec!["-resize".to_owned(), format!("{width}:{height}")]
            // }
            // _ => vec![],
        }
    }
}

impl<T> MagickArgument for Option<T>
where
    T: MagickArgument,
{
    fn get_argument(&self) -> Vec<String> {
        match self {
            Some(t) => t.get_argument(),
            None => vec![],
        }
    }
}

#[derive(Debug)]
pub struct MagickConvertJob {
    pub input_file: String,
    pub output_file: String,
    pub size_arg: Option<ResizeArgument>,
    pub background: Color,
    pub quality: usize,
    pub coalesce: bool,
    pub first_frame: bool,
    pub filter: Option<ResizeFilter>,
    pub resize_arg: Option<ResizeArgument>,
}

pub struct GhostScriptConvertJob {
    pub input_file: String,
    pub output_file: String,
    pub page: usize,
    pub dpi: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobFile {
    pub id: usize,
    pub desired_name: Option<String>,
    pub file_extension: FileType,
}

static FILE_COUNT: AtomicUsize = AtomicUsize::new(0);

impl JobFile {
    pub fn new(file_extension: FileType, desired_name: Option<String>) -> Self {
        FILE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            id: FILE_COUNT.load(Ordering::SeqCst),
            desired_name,
            file_extension,
        }
    }

    pub fn as_filename(&self) -> String {
        match &self.desired_name {
            Some(desired_name) => desired_name.to_string(),
            None => format!(
                "TEMPORARY_CONVERTER_{}.{}",
                self.id,
                self.file_extension.as_extension()
            ),
        }
    }
}

impl GhostScriptConvertJob {
    pub fn get_command(&self) -> Command {
        let mut command = Command::new("gs");

        command
            .arg("-sDEVICE=png16m")
            .arg("-dTextAlphaBits=4")
            .arg(format!("-dFirstPage={}", self.page + 1))
            .arg(format!("-dLastPage={}", self.page + 1))
            .args(vec!["-o", &self.output_file])
            .arg(format!("-r{}", &self.dpi.to_string()))
            .arg(self.input_file.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        command
    }
}

pub enum ConvertJob {
    GhostScript(GhostScriptConvertJob),
    Magick(MagickConvertJob),
}

impl ConvertJob {
    pub fn get_command(&self) -> Command {
        match self {
            ConvertJob::GhostScript(g) => g.get_command(),
            ConvertJob::Magick(g) => g.get_command(),
        }
    }
}

impl MagickConvertJob {
    pub fn get_command(&self) -> Command {
        let mut command = Command::new("magick");

        dbg!(self);

        let size_arg = match self.size_arg {
            None => vec![],
            Some(ResizeArgument::ExactPixels { width, height }) => {
                vec!["-size".to_owned(), format!("{width}x{height}")]
            }
            Some(ResizeArgument::Percentage { width, height: _ }) => {
                let all_pixels = width as f64 / 100.0;
                vec![
                    "-density".to_owned(),
                    ((all_pixels * 96.0) as usize).to_string(),
                ]
            }
        };

        dbg!(&size_arg);

        let resize_arg = match regex::Regex::new(r"svg\[.*\]$")
            .unwrap()
            .is_match(&self.input_file)
        {
            true => vec![],
            false => self.resize_arg.get_argument(),
        };

        if self.first_frame {
            command
                .args(size_arg)
                .args(["-background", &self.background.as_hex_string()])
                .arg(self.input_file.clone())
                .arg("-flatten")
                .args(["-quality".to_string(), format!("{}", self.quality)])
                .args(self.filter.get_argument())
                .args(resize_arg)
                .arg(self.output_file.clone());
        } else {
            command
                .arg(self.input_file.clone())
                .arg("-coalesce")
                .args(vec![
                    "-fill",
                    &self.background.as_hex_string(),
                    "-opaque",
                    "none",
                ])
                .args(vec!["-quality".to_string(), format!("{}", self.quality)])
                .args(self.filter.get_argument())
                .args(resize_arg)
                .arg(self.output_file.clone());
        }

        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        command
    }
}

// pub async fn convert(job: ConvertJob) -> Result<(), String> {
//     let command = job.get_command().await;
//     match command {
//         Ok(output) => match output.status.success() {
//             true => Ok(()),
//             false => Err(std::str::from_utf8(&output.stderr).unwrap().to_owned()),
//         },
//         Err(_) => Err(gettext("Unknown IO error happened")),
//     }
// }

pub async fn wait_for_child(child: std::sync::Arc<SharedChild>) -> Result<(), String> {
    let command = child.wait();
    match command {
        Ok(output) => match output.success() {
            true => Ok(()),
            false => {
                let mut stderr = String::new();
                child
                    .take_stdout()
                    .map(|mut s| s.read_to_string(&mut stderr).ok());
                child
                    .take_stderr()
                    .map(|mut s| s.read_to_string(&mut stderr).ok());
                Err(stderr)
            }
        },
        Err(_) => Err(gettext("Unknown IO error happened")),
    }
}

// pub async fn compress(files: Vec<String>, output: String) -> Result<(), String> {
//     let command = tokio::process::Command::new("zip")
//         .arg("-FSm")
//         .arg(output)
//         .args(files)
//         .output()
//         .await;
//     match command {
//         Ok(output) => match output.status.success() {
//             true => Ok(()),
//             false => Err(std::str::from_utf8(&output.stderr).unwrap().to_owned()),
//         },
//         Err(_) => Err(gettext("Unknown IO error happened")),
//     }
// }

// pub async fn move_files(files: Vec<String>, output_dir: String) -> Result<(), String> {
//     let command = tokio::process::Command::new("mv")
//         .args(files)
//         .arg(output_dir)
//         .output()
//         .await;
//     match command {
//         Ok(output) => match output.status.success() {
//             true => Ok(()),
//             false => Err(std::str::from_utf8(&output.stderr).unwrap().to_owned()),
//         },
//         Err(_) => Err(gettext("Unknown IO error happened")),
//     }
// }

// pub async fn move_file(file: String, target_file: String) -> Result<(), String> {
//     let command = tokio::process::Command::new("mv").arg(file).arg(target_file).output().await;
//     match command {
//         Ok(output) => match output.status.success() {
//             true => Ok(()),
//             false => Err(std::str::from_utf8(&output.stderr).unwrap().to_owned()),
//         },
//         Err(_) => Err(gettext("Unknown IO error happened")),
//     }
// }
