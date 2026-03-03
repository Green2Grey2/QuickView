use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;
use gtk4 as gtk;

use quickview_core::{
    geometry::{Point, Rect},
    ocr::{models::OcrResult, select},
};

#[derive(Default)]
struct State {
    image_width: f64,
    image_height: f64,
    ocr: Option<OcrResult>,

    // Current selection in widget coordinates.
    selecting: bool,
    select_start: Point,
    select_current: Point,

    // Cached selected word indices (into ocr.words)
    selected: Vec<usize>,
}

/// Overlay widget that displays an image and (optionally) an OCR-backed selection layer.
///
/// This is an MVP scaffold:
/// - selection is rectangle drag
/// - selected words are those whose bounding boxes intersect the rectangle
#[derive(Clone)]
pub struct ImageOverlayWidget {
    root: gtk::Overlay,
    picture: gtk::Picture,
    drawing: gtk::DrawingArea,
    spinner: gtk::Spinner,

    state: Rc<RefCell<State>>,
}

impl Default for ImageOverlayWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageOverlayWidget {
    pub fn new() -> Self {
        let root = gtk::Overlay::new();

        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);

        root.set_child(Some(&picture));

        let drawing = gtk::DrawingArea::new();
        drawing.set_hexpand(true);
        drawing.set_vexpand(true);
        drawing.set_focusable(true);
        root.add_overlay(&drawing);

        let spinner = gtk::Spinner::new();
        spinner.set_spinning(false);
        spinner.set_visible(false);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        root.add_overlay(&spinner);

        let state = Rc::new(RefCell::new(State::default()));

        // Draw highlights + selection rectangle.
        {
            let state = state.clone();
            drawing.set_draw_func(move |_, cr, width, height| {
                let s = state.borrow();

                if s.image_width <= 0.0 || s.image_height <= 0.0 {
                    return;
                }

                let (scale, ox, oy) = compute_contain_transform(
                    width as f64,
                    height as f64,
                    s.image_width,
                    s.image_height,
                );

                // Draw selection rectangle (widget coords)
                if s.selecting {
                    let sel = Rect::from_points(s.select_start, s.select_current);
                    cr.set_source_rgba(0.2, 0.6, 1.0, 0.25);
                    cr.rectangle(sel.x, sel.y, sel.w, sel.h);
                    let _ = cr.fill();
                    cr.set_source_rgba(0.2, 0.6, 1.0, 0.8);
                    cr.set_line_width(1.0);
                    cr.rectangle(sel.x, sel.y, sel.w, sel.h);
                    let _ = cr.stroke();
                }

                // Draw selected word boxes
                if let Some(ocr) = &s.ocr {
                    cr.set_source_rgba(1.0, 1.0, 0.0, 0.25);
                    for &idx in &s.selected {
                        if let Some(w) = ocr.words.get(idx) {
                            let rx = ox + w.bbox.x * scale;
                            let ry = oy + w.bbox.y * scale;
                            let rw = w.bbox.w * scale;
                            let rh = w.bbox.h * scale;
                            cr.rectangle(rx, ry, rw, rh);
                            let _ = cr.fill();
                        }
                    }
                }
            });
        }

        // Drag-selection gesture
        {
            let drag = gtk::GestureDrag::new();

            let state_begin = state.clone();
            let drawing_begin = drawing.clone();
            drag.connect_drag_begin(move |_, x, y| {
                let mut s = state_begin.borrow_mut();
                s.selecting = true;
                s.select_start = Point { x, y };
                s.select_current = Point { x, y };
                s.selected.clear();
                drawing_begin.queue_draw();
            });

            let state_update = state.clone();
            let drawing_update = drawing.clone();
            drag.connect_drag_update(move |_, dx, dy| {
                let mut s = state_update.borrow_mut();
                let cur = Point {
                    x: s.select_start.x + dx,
                    y: s.select_start.y + dy,
                };
                s.select_current = cur;

                // Update selection set.
                if let Some(ocr) = &s.ocr {
                    let width = drawing_update.width() as f64;
                    let height = drawing_update.height() as f64;

                    let (scale, ox, oy) =
                        compute_contain_transform(width, height, s.image_width, s.image_height);

                    let sel_widget = Rect::from_points(s.select_start, s.select_current);
                    let sel_image = widget_rect_to_image_rect(sel_widget, scale, ox, oy);

                    let selected = select::select_words(&ocr.words, sel_image)
                        .into_iter()
                        .filter_map(|w| {
                            // Convert reference to index.
                            // This is O(n) but fine for scaffold.
                            ocr.words.iter().position(|x| std::ptr::eq(x, w))
                        })
                        .collect::<Vec<_>>();

                    s.selected = selected;
                }

                drawing_update.queue_draw();
            });

            let state_end = state.clone();
            let drawing_end = drawing.clone();
            drag.connect_drag_end(move |_, _, _| {
                let mut s = state_end.borrow_mut();
                s.selecting = false;
                drawing_end.queue_draw();
            });

            drawing.add_controller(drag);
        }

        Self {
            root,
            picture,
            drawing,
            spinner,
            state,
        }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    pub fn set_texture(&self, texture: gtk::gdk::Texture) {
        let mut s = self.state.borrow_mut();
        s.image_width = texture.width() as f64;
        s.image_height = texture.height() as f64;
        drop(s);
        self.picture.set_paintable(Some(&texture));
        self.drawing.queue_draw();
    }

    pub fn set_ocr_result(&self, result: Option<OcrResult>) {
        let mut s = self.state.borrow_mut();
        s.ocr = result;
        s.selected.clear();
        s.selecting = false;
        self.drawing.queue_draw();
    }

    pub fn set_ocr_busy(&self, busy: bool) {
        self.spinner.set_spinning(busy);
        self.spinner.set_visible(busy);
    }

    pub fn clear_selection(&self) {
        let mut s = self.state.borrow_mut();
        s.selected.clear();
        s.selecting = false;
        self.drawing.queue_draw();
    }

    pub fn selected_text(&self) -> String {
        let s = self.state.borrow();
        let Some(ocr) = &s.ocr else {
            return String::new();
        };

        let words = s
            .selected
            .iter()
            .filter_map(|&idx| ocr.words.get(idx))
            .collect::<Vec<_>>();

        select::selected_text(words)
    }
}

fn compute_contain_transform(
    widget_w: f64,
    widget_h: f64,
    image_w: f64,
    image_h: f64,
) -> (f64, f64, f64) {
    // contain
    let scale = (widget_w / image_w).min(widget_h / image_h).max(0.0001);
    let draw_w = image_w * scale;
    let draw_h = image_h * scale;
    let ox = (widget_w - draw_w) / 2.0;
    let oy = (widget_h - draw_h) / 2.0;
    (scale, ox, oy)
}

fn widget_rect_to_image_rect(sel: Rect, scale: f64, ox: f64, oy: f64) -> Rect {
    Rect {
        x: (sel.x - ox) / scale,
        y: (sel.y - oy) / scale,
        w: sel.w / scale,
        h: sel.h / scale,
    }
}
