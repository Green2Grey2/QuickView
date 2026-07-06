//! Canonical argv codec for single-instance forwarding.
//!
//! With `HANDLES_COMMAND_LINE`, a second invocation's argv is delivered to
//! the primary instance's `command-line` handler. The real CLI (clap, stdin
//! path resolution, canonicalization) runs in the invoking process; what
//! crosses the process boundary is only this fixed, sanitized form:
//!
//! ```text
//! quickview --mode=<quick-preview|full-viewer> --lang=<lang> --file=<abs path>
//! ```
//!
//! Every value is glued to its key in a single `--key=value` token: GLib's
//! local `GOptionContext` pass still runs on the remote side even in
//! pass-through mode, and it strips a bare `--` separator (observed
//! empirically), while unknown `--key=value` tokens travel untouched. The
//! `--file=` framing also keeps file names starting with `-` safe without a
//! separator. Encoding is UTF-8 `String` (gio's `run_with_args` accepts
//! nothing wider); clap's `Option<String>` file argument already rejects
//! non-UTF-8 paths at the outer CLI today, so this codec is not the limiting
//! factor. Decoding takes `OsString` because that is what
//! `ApplicationCommandLine::arguments` hands back.

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use crate::{LaunchOptions, Mode};

const MODE_QUICK_PREVIEW: &str = "quick-preview";
const MODE_FULL_VIEWER: &str = "full-viewer";

pub(crate) fn to_argv(opts: &LaunchOptions) -> Vec<String> {
    let mode = match opts.mode {
        Mode::QuickPreview => MODE_QUICK_PREVIEW,
        Mode::FullViewer => MODE_FULL_VIEWER,
    };
    vec![
        "quickview".to_owned(),
        format!("--mode={mode}"),
        format!("--lang={}", opts.ocr_lang),
        format!("--file={}", opts.file.to_string_lossy()),
    ]
}

pub(crate) fn from_argv(argv: &[OsString]) -> Result<LaunchOptions> {
    let mut mode = None;
    let mut lang = None;
    let mut file: Option<PathBuf> = None;

    for arg in argv.iter().skip(1) {
        // skip program name
        let arg = arg
            .to_str()
            .ok_or_else(|| anyhow!("argument {arg:?} is not UTF-8"))?;
        if let Some(value) = arg.strip_prefix("--mode=") {
            mode = Some(match value {
                MODE_QUICK_PREVIEW => Mode::QuickPreview,
                MODE_FULL_VIEWER => Mode::FullViewer,
                _ => bail!("unknown mode {value:?}"),
            });
        } else if let Some(value) = arg.strip_prefix("--lang=") {
            lang = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--file=") {
            file = Some(PathBuf::from(value));
        } else {
            bail!("unexpected argument {arg:?}");
        }
    }

    Ok(LaunchOptions {
        mode: mode.ok_or_else(|| anyhow!("missing --mode"))?,
        ocr_lang: lang.ok_or_else(|| anyhow!("missing --lang"))?,
        file: file.ok_or_else(|| anyhow!("missing --file"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(mode: Mode, lang: &str, file: &str) -> LaunchOptions {
        LaunchOptions {
            mode,
            file: PathBuf::from(file),
            ocr_lang: lang.to_owned(),
        }
    }

    fn os_argv(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    fn round_trip(original: &LaunchOptions) -> LaunchOptions {
        let argv: Vec<OsString> = to_argv(original).into_iter().map(OsString::from).collect();
        from_argv(&argv).unwrap()
    }

    #[test]
    fn round_trips_both_modes() {
        for mode in [Mode::QuickPreview, Mode::FullViewer] {
            let original = opts(mode, "eng", "/tmp/a.png");
            assert_eq!(round_trip(&original), original);
        }
    }

    #[test]
    fn round_trips_lang_and_awkward_paths() {
        for file in [
            "/tmp/with spaces/shot 1.png",
            "/tmp/-starts-with-dash.png",
            "/tmp/has=equals.png",
        ] {
            let original = opts(Mode::QuickPreview, "deu", file);
            assert_eq!(round_trip(&original), original);
        }
    }

    #[test]
    fn rejects_missing_pieces() {
        assert!(from_argv(&os_argv(&["quickview"])).is_err());
        assert!(from_argv(&os_argv(&["quickview", "--mode=quick-preview"])).is_err());
        assert!(from_argv(&os_argv(&[
            "quickview",
            "--mode=quick-preview",
            "--file=/a"
        ]))
        .is_err());
        assert!(from_argv(&os_argv(&["quickview", "--lang=eng", "--file=/a"])).is_err());
        assert!(from_argv(&os_argv(&[
            "quickview",
            "--mode=quick-preview",
            "--lang=eng"
        ]))
        .is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(from_argv(&os_argv(&["quickview", "--bogus=x"])).is_err());
        assert!(from_argv(&os_argv(&[
            "quickview",
            "--mode=sideways",
            "--lang=eng",
            "--file=/a"
        ]))
        .is_err());
        // A stray positional argument (e.g. a path that lost its --file=
        // framing) must not be silently accepted.
        assert!(from_argv(&os_argv(&[
            "quickview",
            "--mode=full-viewer",
            "--lang=eng",
            "/a"
        ]))
        .is_err());
    }
}
