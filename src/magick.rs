use crate::temp::get_temp_file_path;
use crate::GHOST_SCRIPT_BINARY_NAME;
use crate::{color::Color, filetypes::FileType, window::ResizeFilter};
use gettextrs::gettext;
// use glib::Bytes;
// use gtk::gio::Cancellable;
// use gtk::prelude::{FileExt, InputStreamExt};
use itertools::Itertools;
use shared_child::SharedChild;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

pub async fn count_frames(path: String) -> Result<(usize, Option<(usize, usize)>), ()> {
    let command = tokio::process::Command::new("magick")
        .arg("identify")
        .arg(path)
        .output()
        .await;

    match command {
        Ok(output) => match std::str::from_utf8(&output.stdout) {
            Ok(output_string) => {
                let lines = output_string.lines().collect_vec();
                let count = lines.len();
                let dims = lines
                    .first()
                    .and_then(|line| {
                        let dimension_match = regex::Regex::new(r" \d+x\d+ ").unwrap().find(line);

                        dimension_match.map(|m| {
                            let dims = m
                                .as_str()
                                .trim()
                                .split('x')
                                .map(|n| n.parse::<usize>())
                                .collect_vec();
                            match dims[..] {
                                [Ok(width), Ok(height)] => Some((width, height)),
                                _ => None,
                            }
                        })
                    })
                    .flatten();
                Ok((count, dims))
            }
            _ => Err(()),
        },
        _ => Err(()),
    }
}

// pub async fn pixbuf_bytes(path: String) -> Bytes {
//     let stream = gtk::gio::File::for_path(path)
//         .read(Cancellable::NONE)
//         .unwrap();

//     stream.read_bytes(1073741824, Cancellable::NONE).unwrap()
// }

pub trait MagickArgument {
    fn get_argument(&self) -> Vec<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeArgument {
    Percentage { width: usize, height: usize },
    ExactPixels { width: usize, height: usize },
}

impl Default for ResizeArgument {
    fn default() -> Self {
        Self::Percentage {
            width: 100,
            height: 100,
        }
    }
}

impl MagickArgument for ResizeFilter {
    fn get_argument(&self) -> Vec<String> {
        match self.as_display_string() {
            Some(x) => vec!["-filter".to_string(), x.to_owned()],
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
            ResizeArgument::ExactPixels { width, height } => {
                vec!["-resize".to_owned(), format!("{width}x{height}!")]
            }
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

#[derive(Debug, Clone)]
pub struct MagickConvertJob {
    pub input_file: String,
    pub output_file: String,
    pub background: Color,
    pub quality: usize,
    pub coalesce: bool,
    pub first_frame: bool,
    pub filter: Option<ResizeFilter>,
    pub resize_arg: ResizeArgument,
    pub remove_alpha: bool,
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
        let id = FILE_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            id,
            desired_name,
            file_extension,
        }
    }

    pub fn from_clipboard() -> Self {
        let id = FILE_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            id,
            desired_name: Some(format!("{}.png", gettext("Pasted Image"))),
            file_extension: FileType::Png,
        }
    }

    pub fn as_filename(&self) -> String {
        match &self.desired_name {
            Some(desired_name) => desired_name.to_owned(),
            None => format!(
                "TEMPORARY_SWITCHEROO_{}.{}",
                self.id,
                self.file_extension.as_extension()
            ),
        }
    }
}

impl GhostScriptConvertJob {
    pub fn get_command(&self) -> Command {
        let mut command = Command::new(GHOST_SCRIPT_BINARY_NAME);

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

        let is_svg = self.input_file.ends_with(".svg[0]");

        let (resize_arg, size_arg) = match is_svg {
            true => (
                vec![],
                match self.resize_arg {
                    ResizeArgument::ExactPixels { width, height } => {
                        vec!["-size".to_owned(), format!("{width}x{height}")]
                    }
                    ResizeArgument::Percentage { width, height: _ } => {
                        let all_pixels = width as f64 / 100.0;
                        vec![
                            "-density".to_owned(),
                            ((all_pixels * 96.0) as usize).to_string(),
                        ]
                    }
                },
            ),
            false => (self.resize_arg.get_argument(), vec![]),
        };

        dbg!(&resize_arg);
        dbg!(&size_arg);

        if self.first_frame {
            command
                .args(size_arg)
                .args(["-background", &self.background.as_hex_string()])
                .arg(self.input_file.clone())
                .arg("-flatten");

            if self.remove_alpha {
                command.arg("-alpha").arg("off");
            }

            command
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

pub fn generate_job(
    input_path: &str,
    frame: usize,
    input_type: &FileType,
    output_path: &str,
    output_type: &FileType,
    dir: &TempDir,
    default_arguments: (&MagickConvertJob, &GhostScriptConvertJob),
) -> Vec<ConvertJob> {
    use FileType::*;
    match (input_type, output_type) {
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
            .chain(generate_job(
                &interm,
                0,
                &FileType::Png,
                output_path,
                output_type,
                dir,
                (
                    &MagickConvertJob {
                        resize_arg: ResizeArgument::default(),
                        ..default_arguments.0.to_owned()
                    },
                    default_arguments.1,
                ),
            ))
            .collect()
        }
        (input, output) if input.supports_animation() && output.supports_animation() => {
            std::iter::once(ConvertJob::Magick(MagickConvertJob {
                input_file: input_path.to_owned(),
                output_file: output_path.to_owned(),
                first_frame: false,
                coalesce: false,
                ..*default_arguments.0
            }))
            .collect()
        }
        (input, output) => std::iter::once(ConvertJob::Magick(MagickConvertJob {
            input_file: input_path.to_owned(),
            output_file: output_path.to_owned(),
            first_frame: true,
            coalesce: false,
            remove_alpha: !input.supports_alpha() && output.supports_alpha(),
            ..*default_arguments.0
        }))
        .collect(),
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
