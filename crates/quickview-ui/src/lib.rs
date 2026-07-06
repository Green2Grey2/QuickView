//! GTK4/libadwaita UI for QuickView.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::Result;

use adw::prelude::*;
use gtk4 as gtk;

use gtk::gio;

mod decode;
mod ipc;
pub mod widgets;
pub mod windows;

use windows::shared::ViewerController;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    QuickPreview,
    FullViewer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaunchOptions {
    pub mode: Mode,
    pub file: PathBuf,
    pub ocr_lang: String,
}

/// Windows the primary instance manages across invocations.
///
/// Only the Quick Preview is tracked: it is single-instance (a repeat
/// invocation toggles it closed, FR-002), while full-viewer invocations
/// always open another independent window.
#[derive(Default)]
struct AppState {
    preview: RefCell<Option<PreviewHandle>>,
}

struct PreviewHandle {
    window: glib::WeakRef<gtk::ApplicationWindow>,
    controller: ViewerController,
}

/// Run the GTK application.
///
/// The application is registered with `HANDLES_COMMAND_LINE`, so a second
/// invocation forwards its (pre-resolved) options to the primary instance
/// over the session bus instead of spawning another window stack. clap
/// parsing, stdin reading, and path canonicalization all happen in the
/// invoking process before this point; only the canonical argv built by
/// [`ipc::to_argv`] ever reaches GLib, so GLib never sees the real CLI flags.
pub fn run(opts: LaunchOptions) -> Result<i32> {
    let app = adw::Application::builder()
        .application_id("com.example.QuickView")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    let state = Rc::new(AppState::default());
    app.connect_command_line(move |app, cmdline| {
        // Runs in the primary instance for every invocation (including its
        // own first one): one uniform dispatch path.
        tracing::debug!("command-line argv: {:?}", cmdline.arguments());
        match ipc::from_argv(&cmdline.arguments()) {
            Ok(opts) => {
                dispatch(app, &state, &opts);
                glib::ExitCode::SUCCESS
            }
            Err(err) => {
                // Only reachable through an ipc codec bug or a hand-crafted
                // DBus call; the codec builds every real argv itself.
                tracing::error!("rejected invocation argv: {err:#}");
                glib::ExitCode::from(2)
            }
        }
    });

    let code = app.run_with_args(&ipc::to_argv(&opts));
    Ok(code.into())
}

/// Route one invocation's options to the right window action.
fn dispatch(app: &adw::Application, state: &Rc<AppState>, opts: &LaunchOptions) {
    match opts.mode {
        Mode::QuickPreview => {
            // Clone the live handle out and drop the borrow before closing:
            // `close()` re-enters `close_request`, which mutates the state.
            let existing = state
                .preview
                .borrow()
                .as_ref()
                .and_then(|h| h.window.upgrade().map(|w| (w, h.controller.clone())));

            if let Some((window, controller)) = existing {
                if controller.current_file() == opts.file {
                    // Same file again: the launch keybind acts as a toggle.
                    window.close();
                } else {
                    // Explicit request for a different file: show it, with
                    // the language this invocation asked for.
                    controller.set_ocr_lang(opts.ocr_lang.clone());
                    controller.load_file(&opts.file);
                    window.present();
                }
                return;
            }

            let (window, controller) = windows::quick_preview::present(app, opts);
            let handle = PreviewHandle {
                window: glib::WeakRef::new(),
                controller,
            };
            handle.window.set(Some(&window));
            *state.preview.borrow_mut() = Some(handle);

            let state = Rc::downgrade(state);
            window.connect_close_request(move |_| {
                if let Some(state) = state.upgrade() {
                    *state.preview.borrow_mut() = None;
                }
                glib::Propagation::Proceed
            });
        }
        Mode::FullViewer => {
            // The anchored, keyboard-exclusive preview overlay would sit on
            // top of (and block) the new viewer; close it first.
            let preview = state.preview.borrow_mut().take();
            if let Some(window) = preview.and_then(|h| h.window.upgrade()) {
                window.close();
            }
            windows::full_viewer::present(app, opts);
        }
    }
}
