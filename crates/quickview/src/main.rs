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
    /// Overrides QUICKVIEW_LANG and the config file; defaults to eng.
    #[arg(long)]
    lang: Option<String>,

    /// Directory with .traineddata files, e.g. a tessdata_fast or
    /// tessdata_best checkout. Overrides the config file.
    #[arg(long)]
    tessdata_dir: Option<PathBuf>,

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

    // All resolution happens here, in the invoking process: with the
    // single-instance app, the primary must never re-resolve a remote
    // invocation's environment or config — only fully resolved values cross
    // the process boundary (see quickview-ui's ipc module).
    let ocr = resolve_ocr_options(cli.lang, cli.tessdata_dir);

    let code = quickview_ui::run(quickview_ui::LaunchOptions {
        mode,
        file: file_path,
        ocr,
    })?;

    std::process::exit(code);
}

/// Resolve OCR settings from CLI > env (`QUICKVIEW_LANG`, lang only) >
/// config file > defaults.
///
/// A config file that fails to parse is warned about and treated as absent:
/// a typo must never prevent viewing an image (NFR-004).
fn resolve_ocr_options(
    cli_lang: Option<String>,
    cli_tessdata_dir: Option<PathBuf>,
) -> quickview_core::ocr::tesseract::OcrOptions {
    use quickview_core::config;

    let cfg = config::config_path()
        .map(|path| {
            config::load(&path).unwrap_or_else(|err| {
                tracing::warn!("ignoring config file: {err:#}");
                config::Config::default()
            })
        })
        .unwrap_or_default();

    let env_lang = std::env::var("QUICKVIEW_LANG").ok();
    quickview_core::ocr::tesseract::OcrOptions {
        lang: config::resolve_lang(cli_lang.as_deref(), env_lang.as_deref(), &cfg),
        tessdata_dir: cli_tessdata_dir
            .or_else(|| cfg.ocr.tessdata_dir.clone())
            .map(absolutize),
    }
}

/// Pin a possibly-relative path to this process's cwd.
///
/// Like the image path, the tessdata dir must be absolutized in the invoking
/// process: with the single-instance app, OCR runs in the primary instance,
/// whose cwd has nothing to do with this invocation's. Canonicalizing also
/// resolves symlinks so one directory always hashes to one cache key; a
/// nonexistent path is made absolute without touching the filesystem and
/// fails later in tesseract with a clear error.
fn absolutize(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path)
        .or_else(|_| std::path::absolute(&path))
        .unwrap_or(path)
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

    #[test]
    fn absolutize_resolves_existing_and_pins_missing() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("tessdata");
        std::fs::create_dir(&sub).unwrap();

        // Existing: canonicalized (dot components resolved).
        let dotted = dir.path().join(".").join("tessdata");
        assert_eq!(absolutize(dotted), std::fs::canonicalize(&sub).unwrap());

        // Missing: still made absolute against this process's cwd.
        let missing = absolutize(PathBuf::from("no-such-tessdata"));
        assert!(missing.is_absolute());
        assert!(missing.ends_with("no-such-tessdata"));

        // Already absolute + missing: unchanged.
        assert_eq!(
            absolutize(PathBuf::from("/nonexistent/tessdata")),
            PathBuf::from("/nonexistent/tessdata")
        );
    }
}
