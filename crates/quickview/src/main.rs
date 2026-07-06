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
    let path = match arg.as_deref() {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            let path = buf.lines().next().unwrap_or("").trim();
            if path.is_empty() {
                return Err(anyhow!("No input path provided (stdin was empty)."));
            }
            PathBuf::from(path)
        }
        Some(p) => PathBuf::from(p),
    };
    // Canonicalize at the app boundary so every downstream consumer sees one
    // absolute, symlink-resolved spelling: the OCR cache key must not depend
    // on the invocation cwd (a relative path hashes as its literal string),
    // and directory navigation needs a real parent dir ("image.png" has an
    // empty one). A missing file falls through unchanged so the viewer can
    // show its load-failed state instead of erroring here.
    Ok(std::fs::canonicalize(&path).unwrap_or(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_canonicalizes_existing_paths() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let img = dir.path().join("a.png");
        std::fs::write(&img, b"x").unwrap();

        // A dot-and-dotdot spelling of the same file resolves to one form.
        let indirect = dir.path().join(".").join("sub").join("..").join("a.png");
        let resolved = resolve_input_path(Some(indirect.display().to_string())).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&img).unwrap());
        assert!(resolved.is_absolute());
    }

    #[test]
    fn resolve_passes_missing_paths_through() {
        let p = resolve_input_path(Some("does-not-exist.png".into())).unwrap();
        assert_eq!(p, PathBuf::from("does-not-exist.png"));
    }
}
