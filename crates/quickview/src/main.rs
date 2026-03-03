use std::{
    io::{self, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "quickview",
    version,
    about = "Wayland image viewer with OCR text selection"
)]
struct Cli {
    /// Launch borderless Quick Preview window.
    #[arg(long)]
    quick_preview: bool,

    /// OCR language (Tesseract -l). Example: eng, deu, spa.
    #[arg(long, default_value = "eng")]
    lang: String,

    /// Image file path. Use '-' (or omit) to read a path from stdin.
    file: Option<String>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let file_path = resolve_input_path(cli.file)?;

    let mode = if cli.quick_preview {
        quickview_ui::Mode::QuickPreview
    } else {
        quickview_ui::Mode::FullViewer
    };

    let code = quickview_ui::run(quickview_ui::LaunchOptions {
        mode,
        file: file_path,
        ocr_lang: cli.lang,
    })?;

    std::process::exit(code);
}

fn resolve_input_path(arg: Option<String>) -> Result<PathBuf> {
    match arg.as_deref() {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            let path = buf.lines().next().unwrap_or("").trim();
            if path.is_empty() {
                return Err(anyhow!("No input path provided (stdin was empty)."));
            }
            Ok(PathBuf::from(path))
        }
        Some(p) => Ok(PathBuf::from(p)),
    }
}
