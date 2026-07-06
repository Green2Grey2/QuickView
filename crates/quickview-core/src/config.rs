//! User configuration (`~/.config/quickview/config.toml`).
//!
//! Resolution precedence for every setting: CLI flag > environment variable
//! (lang only: `QUICKVIEW_LANG`) > config file > built-in default. The
//! precedence logic lives in pure functions here so it is testable headlessly;
//! reading the actual environment is the caller's job.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;

/// Built-in default OCR language (Tesseract `-l`).
pub const DEFAULT_OCR_LANG: &str = "eng";

/// Return the per-project directories for QuickView.
///
/// The triple must stay in sync with the application ID
/// `io.github.Green2Grey2.QuickView`. On Linux the derived paths only use the
/// lowercased app name (`~/.cache/quickview/`, `~/.config/quickview/`).
pub(crate) fn project_dirs() -> Option<ProjectDirs> {
    // qualifier, org, app
    ProjectDirs::from("io.github", "Green2Grey2", "QuickView")
}

/// Path of the config file (`~/.config/quickview/config.toml` on Linux).
pub fn config_path() -> Option<PathBuf> {
    Some(project_dirs()?.config_dir().join("config.toml"))
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub ocr: OcrSection,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OcrSection {
    /// Default OCR language (overridden by `QUICKVIEW_LANG` and `--lang`).
    pub lang: Option<String>,
    /// Directory with `.traineddata` files, e.g. a `tessdata_fast` or
    /// `tessdata_best` checkout (overridden by `--tessdata-dir`).
    pub tessdata_dir: Option<PathBuf>,
    /// Maximum image dimension before OCR runs on a downscaled copy
    /// (overridden by `--max-ocr-dim`; `0` disables the guardrail).
    pub max_dimension: Option<u32>,
}

/// Load the config at `path`.
///
/// A missing file is a normal, empty configuration. A file that exists but
/// does not parse (including unknown fields, which usually mean a typo) is an
/// `Err` so the caller can warn — a broken config must never prevent viewing
/// an image, but it must not be silently ignored either.
pub fn load(path: &Path) -> Result<Config> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => return Err(err).with_context(|| format!("failed to read {}", path.display())),
    };
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

/// Resolve the effective OCR language.
///
/// Precedence: CLI flag > env (`QUICKVIEW_LANG`) > config file > `"eng"`.
/// Blank or whitespace-only values are treated as unset at each level.
pub fn resolve_lang(cli: Option<&str>, env: Option<&str>, config: &Config) -> String {
    [cli, env, config.ocr.lang.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or(DEFAULT_OCR_LANG)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_lang(lang: &str) -> Config {
        Config {
            ocr: OcrSection {
                lang: Some(lang.to_owned()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn lang_precedence_cli_env_config_default() {
        let cfg = config_with_lang("fra");
        assert_eq!(resolve_lang(Some("deu"), Some("spa"), &cfg), "deu");
        assert_eq!(resolve_lang(None, Some("spa"), &cfg), "spa");
        assert_eq!(resolve_lang(None, None, &cfg), "fra");
        assert_eq!(resolve_lang(None, None, &Config::default()), "eng");
    }

    #[test]
    fn blank_values_are_unset_at_each_level() {
        let cfg = config_with_lang("fra");
        assert_eq!(resolve_lang(Some(""), Some("  "), &cfg), "fra");
        assert_eq!(resolve_lang(Some(" "), None, &config_with_lang("")), "eng");
    }

    #[test]
    fn load_missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = load(&dir.path().join("nope.toml")).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn load_parses_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[ocr]\nlang = \"deu\"\n").unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.ocr.lang.as_deref(), Some("deu"));
        assert_eq!(cfg.ocr.tessdata_dir, None);

        std::fs::write(
            &path,
            "[ocr]\ntessdata_dir = \"/opt/tessdata_fast\"\nmax_dimension = 4000\n",
        )
        .unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.ocr.lang, None);
        assert_eq!(
            cfg.ocr.tessdata_dir.as_deref(),
            Some(Path::new("/opt/tessdata_fast"))
        );
        assert_eq!(cfg.ocr.max_dimension, Some(4000));
    }

    #[test]
    fn load_rejects_garbage_and_typos() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        std::fs::write(&path, "not toml [ at all").unwrap();
        assert!(load(&path).is_err());

        // Unknown field: almost certainly a typo the user wants to hear about.
        std::fs::write(&path, "[ocr]\nlanguage = \"deu\"\n").unwrap();
        assert!(load(&path).is_err());
    }
}
