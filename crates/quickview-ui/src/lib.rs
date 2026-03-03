//! GTK4/libadwaita UI for QuickView.

use std::path::PathBuf;

use anyhow::Result;

use adw::prelude::*;

pub mod widgets;
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    QuickPreview,
    FullViewer,
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub mode: Mode,
    pub file: PathBuf,
    pub ocr_lang: String,
}

/// Run the GTK application.
///
/// Notes:
/// - We intentionally call `run_with_args(&[])` so GLib/GTK does not reject our custom CLI flags.
pub fn run(opts: LaunchOptions) -> Result<i32> {
    let app = adw::Application::builder()
        .application_id("com.example.QuickView")
        .build();

    let opts_clone = opts.clone();
    app.connect_activate(move |app| match opts_clone.mode {
        Mode::QuickPreview => {
            windows::quick_preview::present(app, &opts_clone);
        }
        Mode::FullViewer => {
            windows::full_viewer::present(app, &opts_clone);
        }
    });

    // Important: don't pass our CLI args to GTK.
    let code = app.run_with_args::<glib::GString>(&[]);
    Ok(code.into())
}
