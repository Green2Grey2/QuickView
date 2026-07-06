use gtk4 as gtk;

use adw::prelude::*;
use gtk::prelude::WidgetExt;

use crate::{
    windows::shared::{FileInfo, ViewerController},
    LaunchOptions,
};

pub fn present(app: &adw::Application, opts: &LaunchOptions) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("QuickView")
        .default_width(1100)
        .default_height(800)
        .build();

    let title = adw::WindowTitle::new("QuickView", "");
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));

    let viewer = ViewerController::new(opts.file.clone(), opts.ocr_lang.clone());

    {
        let title = title.clone();
        viewer.connect_file_loaded(move |info| {
            title.set_title(&info.name);
            title.set_subtitle(&format_subtitle(info));
        });
    }

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&viewer.widget()));

    adw::prelude::AdwApplicationWindowExt::set_content(&window, Some(&toolbar_view));

    // Key handling: arrows navigate, Ctrl+C copies.
    {
        let viewer = viewer.clone();
        let overlay = viewer.overlay();
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

/// Subtitle string for the headerbar: `1920×1080 · 2.4 MB`.
fn format_subtitle(info: &FileInfo) -> String {
    if info.load_failed {
        return "Could not load image".to_string();
    }

    let dims = format!("{}×{}", info.width, info.height);
    match info.size_bytes {
        Some(bytes) => format!("{dims} · {}", glib::format_size(bytes)),
        None => dims,
    }
}

#[cfg(test)]
mod tests {
    use super::{format_subtitle, FileInfo};

    fn info(width: i32, height: i32, size_bytes: Option<u64>, load_failed: bool) -> FileInfo {
        FileInfo {
            name: "test.png".to_string(),
            width,
            height,
            size_bytes,
            load_failed,
        }
    }

    #[test]
    fn subtitle_shows_dimensions_and_size() {
        let subtitle = format_subtitle(&info(1920, 1080, Some(2_400_000), false));
        let expected_size = glib::format_size(2_400_000);
        assert_eq!(subtitle, format!("1920×1080 · {expected_size}"));
    }

    #[test]
    fn subtitle_omits_size_when_metadata_unavailable() {
        assert_eq!(format_subtitle(&info(800, 600, None, false)), "800×600");
    }

    #[test]
    fn subtitle_handles_zero_bytes() {
        let subtitle = format_subtitle(&info(1, 1, Some(0), false));
        let expected_size = glib::format_size(0);
        assert_eq!(subtitle, format!("1×1 · {expected_size}"));
    }

    #[test]
    fn subtitle_handles_large_files() {
        let subtitle = format_subtitle(&info(12000, 8000, Some(3_500_000_000), false));
        let expected_size = glib::format_size(3_500_000_000);
        assert_eq!(subtitle, format!("12000×8000 · {expected_size}"));
    }

    #[test]
    fn subtitle_reports_load_failure() {
        assert_eq!(
            format_subtitle(&info(0, 0, Some(123), true)),
            "Could not load image"
        );
    }
}
