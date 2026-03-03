use gtk4 as gtk;

use adw::prelude::*;
use gtk::prelude::WidgetExt;

use crate::{windows::shared::ViewerController, LaunchOptions};

pub fn present(app: &adw::Application, opts: &LaunchOptions) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("QuickView")
        .default_width(1100)
        .default_height(800)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("QuickView"))));

    let viewer = ViewerController::new(opts.file.clone(), opts.ocr_lang.clone());

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&viewer.widget()));

    adw::prelude::AdwApplicationWindowExt::set_content(&window, Some(&toolbar_view));

    // Key handling: arrows navigate, Ctrl+C copies.
    {
        let viewer = viewer.clone();
        let overlay = viewer.overlay();
        let window_clone = window.clone();
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, state| {
            let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);

            if is_ctrl && key == gtk::gdk::Key::c {
                let display = WidgetExt::display(&window_clone);
                viewer.copy_selection_to_clipboard(&display);
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

            if key == gtk::gdk::Key::Left {
                viewer.prev_image();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Right {
                viewer.next_image();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        window.add_controller(controller);
    }

    window.present();
}
