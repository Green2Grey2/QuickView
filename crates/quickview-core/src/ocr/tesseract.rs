use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context, Result};

/// Settings that influence what tesseract recognizes.
///
/// Every field here changes OCR output, so every field must join the cache
/// key (ADR-0009): see [`crate::cache::ocr_cache_path`].
#[derive(Debug, Clone, PartialEq)]
pub struct OcrOptions {
    /// Language(s), tesseract `-l` (e.g. `eng`, `deu`, `eng+deu`).
    pub lang: String,
    /// Directory with `.traineddata` files (`--tessdata-dir`), e.g. a
    /// `tessdata_fast` or `tessdata_best` checkout. `None` uses the system
    /// default.
    pub tessdata_dir: Option<PathBuf>,
}

/// Run tesseract and return TSV output (written to stdout).
///
/// Command used (baseline):
/// `tesseract <input> - -l <lang> [--tessdata-dir <dir>] tsv quiet`
pub fn run_tesseract_tsv(input: &Path, opts: &OcrOptions) -> Result<String> {
    let output = Command::new("tesseract")
        .args(tesseract_args(input, opts))
        .output()
        .context("failed to spawn tesseract; is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("tesseract failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout).context("tesseract output not valid UTF-8")?;
    Ok(stdout)
}

/// Build the tesseract argv. Options must precede the trailing `tsv quiet`
/// config names or tesseract treats them as config files.
fn tesseract_args(input: &Path, opts: &OcrOptions) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![input.into(), "-".into(), "-l".into(), (&opts.lang).into()];
    if let Some(dir) = &opts.tessdata_dir {
        args.push("--tessdata-dir".into());
        args.push(dir.into());
    }
    args.push("tsv".into());
    args.push("quiet".into());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(lang: &str, tessdata_dir: Option<&str>) -> OcrOptions {
        OcrOptions {
            lang: lang.to_owned(),
            tessdata_dir: tessdata_dir.map(PathBuf::from),
        }
    }

    #[test]
    fn args_baseline() {
        let args = tesseract_args(Path::new("/tmp/a.png"), &opts("eng", None));
        assert_eq!(args, ["/tmp/a.png", "-", "-l", "eng", "tsv", "quiet"]);
    }

    #[test]
    fn args_with_tessdata_dir_precede_config_names() {
        let args = tesseract_args(
            Path::new("/tmp/a.png"),
            &opts("deu", Some("/opt/tessdata_fast")),
        );
        assert_eq!(
            args,
            [
                "/tmp/a.png",
                "-",
                "-l",
                "deu",
                "--tessdata-dir",
                "/opt/tessdata_fast",
                "tsv",
                "quiet"
            ]
        );
    }
}
