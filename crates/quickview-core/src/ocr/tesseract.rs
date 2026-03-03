use std::{path::Path, process::Command};

use anyhow::{anyhow, Context, Result};

/// Run tesseract and return TSV output (written to stdout).
///
/// Command used (baseline):
/// `tesseract <input> - -l <lang> tsv quiet`
pub fn run_tesseract_tsv(input: &Path, lang: &str) -> Result<String> {
    let output = Command::new("tesseract")
        .arg(input)
        .arg("-")
        .arg("-l")
        .arg(lang)
        .arg("tsv")
        .arg("quiet")
        .output()
        .context("failed to spawn tesseract; is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("tesseract failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout).context("tesseract output not valid UTF-8")?;
    Ok(stdout)
}
