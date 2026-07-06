use std::{cell::Cell, rc::Rc};

use gtk4 as gtk;

use adw::prelude::*;
use gtk::prelude::WidgetExt;

use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::{windows::shared::ViewerController, LaunchOptions};

/// Size of the centered preview panel on the layer-shell path, and of the
/// whole window on the fallback path.
const PANEL_WIDTH: i32 = 900;
const PANEL_HEIGHT: i32 = 700;

pub fn present(
    app: &adw::Application,
    opts: &LaunchOptions,
) -> (gtk::ApplicationWindow, ViewerController) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("QuickView")
        .decorated(false)
        .resizable(true)
        .default_width(PANEL_WIDTH)
        .default_height(PANEL_HEIGHT)
        .build();

    let viewer = ViewerController::new(opts.file.clone(), opts.ocr_lang.clone());

    // Layer shell if supported.
    if gtk4_layer_shell::is_supported() {
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_namespace(Some("quickview"));
        // Anchored to all edges: the surface covers the output so clicks
        // outside the centered panel land on our transparent backdrop and
        // dismiss the preview (FR-002). While the preview is open it therefore
        // owns all pointer input on this output — intended Quick Look-style
        // modality.
        // Not exclusive: do not reserve screen space.
        window.set_exclusive_zone(0);
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
        }

        ensure_css_installed();
        window.add_css_class("qv-preview-surface");

        // The panel is a *sibling* of the backdrop inside a gtk::Overlay, so
        // clicks on the image never reach the backdrop gesture — drag-select
        // and middle-drag pan keep working. Do not attach the close gesture
        // to the window itself.
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.set_hexpand(true);
        backdrop.set_vexpand(true);
        backdrop.add_css_class("qv-backdrop");
        {
            let window = window.clone();
            let click = gtk::GestureClick::new();
            click.set_button(0); // any button
            click.connect_pressed(move |_, _, _, _| window.close());
            backdrop.add_controller(click);
        }

        // The canvas reports the image's natural size, and gtk::Overlay
        // allocates centered overlay children at natural size — without a
        // clamp a large image would grow the panel over the whole surface,
        // leaving no backdrop to click. set_size_request is only a minimum;
        // the clamps cap the maximum, so the panel is exactly
        // PANEL_WIDTH x PANEL_HEIGHT and the canvas letterboxes inside it.
        // tightening_threshold == maximum_size: with the default (400) the
        // clamp's own natural size eases out well past maximum to give the
        // child a run-in, which the Overlay would happily allocate.
        let vclamp = adw::Clamp::builder()
            .maximum_size(PANEL_HEIGHT)
            .tightening_threshold(PANEL_HEIGHT)
            .orientation(gtk::Orientation::Vertical)
            .child(&viewer.widget())
            .build();
        let hclamp = adw::Clamp::builder()
            .maximum_size(PANEL_WIDTH)
            .tightening_threshold(PANEL_WIDTH)
            .child(&vclamp)
            .build();

        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.set_halign(gtk::Align::Center);
        panel.set_valign(gtk::Align::Center);
        panel.set_size_request(PANEL_WIDTH, PANEL_HEIGHT);
        panel.add_css_class("qv-preview-panel");
        panel.append(&hclamp);

        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&backdrop));
        overlay.add_overlay(&panel);
        window.set_child(Some(&overlay));
    } else {
        window.set_child(Some(&viewer.widget()));

        // No layer shell: there is nothing "outside" the window for us to
        // observe clicks on under Wayland, so dismiss on focus loss instead.
        // The latch only closes on a true->false transition, guarding against
        // compositors that map the window unfocused.
        let was_active = Rc::new(Cell::new(false));
        window.connect_is_active_notify(move |window| {
            if window.is_active() {
                was_active.set(true);
            } else if was_active.get() {
                window.close();
            }
        });
    }

    // Key handling: Esc/Space closes, Ctrl+C copies.
    {
        let viewer = viewer.clone();
        let overlay = viewer.overlay();
        let window_clone = window.clone();
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, state| {
            let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            if is_ctrl && key == gtk::gdk::Key::c {
                viewer.copy_selection_to_clipboard();
                return glib::Propagation::Stop;
            }

            let is_shift = state.contains(gtk::gdk::ModifierType::SHIFT_MASK);
            if key == gtk::gdk::Key::Menu || (is_shift && key == gtk::gdk::Key::F10) {
                overlay.open_context_menu();
                return glib::Propagation::Stop;
            }

            if key == gtk::gdk::Key::plus
                || key == gtk::gdk::Key::equal
                || key == gtk::gdk::Key::KP_Add
            {
                overlay.zoom_in();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::minus || key == gtk::gdk::Key::KP_Subtract {
                overlay.zoom_out();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::_0 || key == gtk::gdk::Key::Home {
                overlay.reset_view();
                return glib::Propagation::Stop;
            }

            if key == gtk::gdk::Key::Escape || key == gtk::gdk::Key::space {
                window_clone.close();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        window.add_controller(controller);
    }

    window.present();
    (window, viewer)
}

/// Install the preview surface CSS once per (main-thread) session.
///
/// The window itself must be transparent so the all-edges layer surface does
/// not paint the theme background across the whole output; the panel then
/// restores an opaque background behind the image.
fn ensure_css_installed() {
    thread_local! {
        static INSTALLED: Cell<bool> = const { Cell::new(false) };
    }
    if INSTALLED.get() {
        return;
    }
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "window.qv-preview-surface { background: transparent; }\n\
         .qv-preview-panel { background-color: @window_bg_color; border-radius: 12px; }",
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    INSTALLED.set(true);
}
