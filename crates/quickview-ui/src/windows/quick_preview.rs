use gtk4 as gtk;

use adw::prelude::*;
use gtk::prelude::WidgetExt;

use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

use crate::{windows::shared::ViewerController, LaunchOptions};

pub fn present(app: &adw::Application, opts: &LaunchOptions) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("QuickView")
        .decorated(false)
        .resizable(true)
        .default_width(900)
        .default_height(700)
        .build();

    // Layer shell if supported.
    if gtk4_layer_shell::is_supported() {
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_namespace(Some("quickview"));
        // Not anchored: centered by compositor.
        // Not exclusive: do not reserve screen space.
        window.set_exclusive_zone(0);
    }

    let viewer = ViewerController::new(opts.file.clone(), opts.ocr_lang.clone());
    window.set_child(Some(&viewer.widget()));

    // Key handling: Esc/Space closes, Ctrl+C copies.
    {
        let viewer = viewer.clone();
        let window_clone = window.clone();
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, state| {
            let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            if is_ctrl && key == gtk::gdk::Key::c {
                let display = WidgetExt::display(&window_clone);
                viewer.copy_selection_to_clipboard(&display);
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
}
