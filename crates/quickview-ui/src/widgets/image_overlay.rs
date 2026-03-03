use std::cell::RefCell;

use glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use quickview_core::{
    geometry::{Point, Rect, ViewTransform},
    ocr::{models::OcrResult, select},
};

const MIN_ZOOM_FACTOR: f64 = 1.0;
const BASE_MAX_ZOOM_FACTOR: f64 = 20.0;
const ZOOM_STEP: f64 = 1.25;
const INTEGER_SCALE_EPS: f64 = 0.02;
const PAN_DIM_EPS: f64 = 0.5;

#[derive(Clone)]
pub struct ImageOverlayWidget {
    root: gtk::Overlay,
    canvas: ZoomableCanvas,
    spinner: gtk::Spinner,
}

impl Default for ImageOverlayWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageOverlayWidget {
    pub fn new() -> Self {
        let root = gtk::Overlay::new();

        let canvas = ZoomableCanvas::new();
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);
        root.set_child(Some(&canvas));

        let spinner = gtk::Spinner::new();
        spinner.set_spinning(false);
        spinner.set_visible(false);
        spinner.set_halign(gtk::Align::Center);
        spinner.set_valign(gtk::Align::Center);
        root.add_overlay(&spinner);

        Self {
            root,
            canvas,
            spinner,
        }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    pub fn set_texture(&self, texture: gtk::gdk::Texture) {
        self.canvas.set_texture(texture);
    }

    pub fn set_ocr_result(&self, result: Option<OcrResult>) {
        self.canvas.set_ocr_result(result);
    }

    pub fn set_ocr_busy(&self, busy: bool) {
        self.spinner.set_spinning(busy);
        self.spinner.set_visible(busy);
    }

    pub fn clear_selection(&self) {
        self.canvas.clear_selection();
    }

    pub fn selected_text(&self) -> String {
        self.canvas.selected_text()
    }

    pub fn zoom_by(&self, factor: f64) {
        self.canvas.zoom_by(factor);
    }

    pub fn zoom_in(&self) {
        self.canvas.zoom_by(ZOOM_STEP);
    }

    pub fn zoom_out(&self) {
        self.canvas.zoom_by(1.0 / ZOOM_STEP);
    }

    pub fn reset_view(&self) {
        self.canvas.reset_view();
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ZoomableCanvas {
        pub(super) state: RefCell<CanvasState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ZoomableCanvas {
        const NAME: &'static str = "QuickViewZoomableCanvas";
        type Type = super::ZoomableCanvas;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for ZoomableCanvas {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_focusable(true);
            obj.setup_controllers();
        }
    }

    impl WidgetImpl for ZoomableCanvas {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let state = self.state.borrow();
            let natural = natural_size_for_measure(
                orientation,
                for_size,
                state.image_width,
                state.image_height,
            );
            (1, natural, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.parent_snapshot(snapshot);

            let widget = self.obj();
            let widget_w = widget.width() as f64;
            let widget_h = widget.height() as f64;

            let mut state = self.state.borrow_mut();
            let texture = state.texture.clone();
            let Some(transform) = transform_for_widget(&mut state, widget_w, widget_h) else {
                return;
            };

            let Some(texture) = texture.as_ref() else {
                return;
            };

            let bounds = gtk::graphene::Rect::new(
                transform.offset_x as f32,
                transform.offset_y as f32,
                (state.image_width * transform.scale) as f32,
                (state.image_height * transform.scale) as f32,
            );
            snapshot.append_scaled_texture(
                texture,
                scaling_filter_for_scale(transform.scale),
                &bounds,
            );

            let overlay_bounds =
                gtk::graphene::Rect::new(0.0, 0.0, widget_w as f32, widget_h as f32);
            let cr = snapshot.append_cairo(&overlay_bounds);

            if state.selecting {
                let sel = Rect::from_points(state.select_start_widget, state.select_current_widget);
                cr.set_source_rgba(0.2, 0.6, 1.0, 0.25);
                cr.rectangle(sel.x, sel.y, sel.w, sel.h);
                let _ = cr.fill();

                cr.set_source_rgba(0.2, 0.6, 1.0, 0.8);
                cr.set_line_width(1.0);
                cr.rectangle(sel.x, sel.y, sel.w, sel.h);
                let _ = cr.stroke();
            }

            if let Some(ocr) = &state.ocr {
                cr.set_source_rgba(1.0, 1.0, 0.0, 0.25);
                for &idx in &state.selected_indices {
                    if let Some(word) = ocr.words.get(idx) {
                        let rect = transform.image_rect_to_widget(word.bbox);
                        cr.rectangle(rect.x, rect.y, rect.w, rect.h);
                        let _ = cr.fill();
                    }
                }
            }
        }
    }

    #[derive(Clone)]
    pub(super) struct CanvasState {
        pub(super) texture: Option<gtk::gdk::Texture>,
        pub(super) image_width: f64,
        pub(super) image_height: f64,
        pub(super) ocr: Option<OcrResult>,
        pub(super) selected_indices: Vec<usize>,

        pub(super) zoom_factor: f64,
        pub(super) center_img: Point,

        pub(super) selecting: bool,
        pub(super) select_start_widget: Point,
        pub(super) select_current_widget: Point,

        pub(super) panning: bool,
        pub(super) pan_start_widget: Point,
        pub(super) pan_start_center_img: Point,
        pub(super) last_cursor_widget: Option<Point>,

        pub(super) pinch_active: bool,
        pub(super) pinch_start_zoom_factor: f64,
        pub(super) pinch_start_center_img: Point,
        pub(super) pinch_anchor_widget: Point,
    }

    impl Default for CanvasState {
        fn default() -> Self {
            Self {
                texture: None,
                image_width: 0.0,
                image_height: 0.0,
                ocr: None,
                selected_indices: Vec::new(),
                zoom_factor: 1.0,
                center_img: Point::default(),
                selecting: false,
                select_start_widget: Point::default(),
                select_current_widget: Point::default(),
                panning: false,
                pan_start_widget: Point::default(),
                pan_start_center_img: Point::default(),
                last_cursor_widget: None,
                pinch_active: false,
                pinch_start_zoom_factor: 1.0,
                pinch_start_center_img: Point::default(),
                pinch_anchor_widget: Point::default(),
            }
        }
    }
}

glib::wrapper! {
    pub struct ZoomableCanvas(ObjectSubclass<imp::ZoomableCanvas>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ZoomableCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoomableCanvas {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_texture(&self, texture: gtk::gdk::Texture) {
        let mut state = self.imp().state.borrow_mut();
        state.texture = Some(texture.clone());
        state.image_width = texture.width() as f64;
        state.image_height = texture.height() as f64;
        state.zoom_factor = MIN_ZOOM_FACTOR;
        state.center_img = Point {
            x: state.image_width * 0.5,
            y: state.image_height * 0.5,
        };
        state.selecting = false;
        state.panning = false;
        state.pinch_active = false;
        state.selected_indices.clear();
        drop(state);
        self.queue_draw();
        self.update_cursor();
    }

    pub fn set_ocr_result(&self, result: Option<OcrResult>) {
        let mut state = self.imp().state.borrow_mut();
        state.ocr = result;
        state.selected_indices.clear();
        state.selecting = false;
        drop(state);
        self.queue_draw();
    }

    pub fn clear_selection(&self) {
        let mut state = self.imp().state.borrow_mut();
        state.selecting = false;
        state.selected_indices.clear();
        drop(state);
        self.queue_draw();
    }

    pub fn selected_text(&self) -> String {
        let state = self.imp().state.borrow();
        let Some(ocr) = &state.ocr else {
            return String::new();
        };

        let words = state
            .selected_indices
            .iter()
            .filter_map(|&idx| ocr.words.get(idx))
            .collect::<Vec<_>>();
        select::selected_text(words)
    }

    pub fn zoom_by(&self, factor: f64) {
        if factor <= 0.0 {
            return;
        }
        self.zoom_at(self.widget_center(), factor);
    }

    pub fn reset_view(&self) {
        let mut state = self.imp().state.borrow_mut();
        if state.image_width <= 0.0 || state.image_height <= 0.0 {
            return;
        }
        state.zoom_factor = MIN_ZOOM_FACTOR;
        state.center_img = Point {
            x: state.image_width * 0.5,
            y: state.image_height * 0.5,
        };
        state.selecting = false;
        state.panning = false;
        state.pinch_active = false;
        drop(state);
        self.queue_draw();
        self.update_cursor();
    }

    fn setup_controllers(&self) {
        let motion = gtk::EventControllerMotion::new();
        {
            let canvas = self.clone();
            motion.connect_motion(move |_, x, y| {
                let mut state = canvas.imp().state.borrow_mut();
                state.last_cursor_widget = Some(Point { x, y });
                drop(state);
                canvas.update_cursor();
            });
        }
        {
            let canvas = self.clone();
            motion.connect_leave(move |_| {
                let mut state = canvas.imp().state.borrow_mut();
                state.last_cursor_widget = None;
                drop(state);
                canvas.update_cursor();
            });
        }
        self.add_controller(motion);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        {
            let canvas = self.clone();
            scroll.connect_scroll(move |controller, _dx, dy| {
                let mods = controller.current_event_state();
                if !mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                    return glib::Propagation::Proceed;
                }

                let factor = if dy < 0.0 {
                    ZOOM_STEP
                } else if dy > 0.0 {
                    1.0 / ZOOM_STEP
                } else {
                    return glib::Propagation::Stop;
                };

                let anchor = canvas.last_cursor_widget();
                canvas.zoom_at(anchor, factor);
                glib::Propagation::Stop
            });
        }
        self.add_controller(scroll);

        let pinch = gtk::GestureZoom::new();
        {
            let canvas = self.clone();
            pinch.connect_begin(move |gesture, _| {
                canvas.on_pinch_begin(gesture);
            });
        }
        {
            let canvas = self.clone();
            pinch.connect_scale_changed(move |gesture, scale_factor| {
                canvas.on_pinch_scale_changed(gesture, scale_factor);
            });
        }
        {
            let canvas = self.clone();
            pinch.connect_end(move |_, _| {
                canvas.on_pinch_end();
            });
        }
        {
            let canvas = self.clone();
            pinch.connect_cancel(move |_, _| {
                canvas.on_pinch_end();
            });
        }
        self.add_controller(pinch);

        let drag = gtk::GestureDrag::new();
        drag.set_button(0);
        {
            let canvas = self.clone();
            drag.connect_drag_begin(move |gesture, x, y| {
                canvas.on_drag_begin(gesture, x, y);
            });
        }
        {
            let canvas = self.clone();
            drag.connect_drag_update(move |_, dx, dy| {
                canvas.on_drag_update(dx, dy);
            });
        }
        {
            let canvas = self.clone();
            drag.connect_drag_end(move |_, _, _| {
                canvas.on_drag_end();
            });
        }
        self.add_controller(drag);
    }

    fn on_drag_begin(&self, gesture: &gtk::GestureDrag, x: f64, y: f64) {
        let mut state = self.imp().state.borrow_mut();
        let widget_w = self.width() as f64;
        let widget_h = self.height() as f64;

        let cursor = Point { x, y };
        state.last_cursor_widget = Some(cursor);

        let button = gesture.current_button();
        let mods = gesture.current_event_state();
        let ctrl_pressed = mods.contains(gtk::gdk::ModifierType::CONTROL_MASK);
        let pan_requested = button == gtk::gdk::BUTTON_MIDDLE
            || (button == gtk::gdk::BUTTON_PRIMARY && ctrl_pressed);
        let can_pan = can_pan_at_view(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            state.zoom_factor,
        );

        if pan_requested && can_pan {
            state.panning = true;
            state.selecting = false;
            state.pan_start_widget = cursor;
            state.pan_start_center_img = state.center_img;
        } else if button == gtk::gdk::BUTTON_PRIMARY {
            state.selecting = true;
            state.panning = false;
            state.select_start_widget = cursor;
            state.select_current_widget = cursor;
            state.selected_indices.clear();
        } else {
            state.selecting = false;
            state.panning = false;
        }

        drop(state);
        self.update_cursor();
        self.queue_draw();
    }

    fn on_drag_update(&self, dx: f64, dy: f64) {
        let mut needs_redraw = false;
        let mut state = self.imp().state.borrow_mut();
        let widget_w = self.width() as f64;
        let widget_h = self.height() as f64;

        if state.panning {
            let current = Point {
                x: state.pan_start_widget.x + dx,
                y: state.pan_start_widget.y + dy,
            };
            state.last_cursor_widget = Some(current);

            if let Some(transform) = transform_for_widget(&mut state, widget_w, widget_h) {
                state.center_img = Point {
                    x: state.pan_start_center_img.x - (dx / transform.scale),
                    y: state.pan_start_center_img.y - (dy / transform.scale),
                };
                state.center_img = ViewTransform::clamp_center(
                    widget_w,
                    widget_h,
                    state.image_width,
                    state.image_height,
                    transform.scale,
                    state.center_img,
                );
                needs_redraw = true;
            }
        }

        if state.selecting {
            let current = Point {
                x: state.select_start_widget.x + dx,
                y: state.select_start_widget.y + dy,
            };
            state.select_current_widget = current;
            state.last_cursor_widget = Some(current);

            if let Some(transform) = transform_for_widget(&mut state, widget_w, widget_h) {
                let sel_widget =
                    Rect::from_points(state.select_start_widget, state.select_current_widget);
                let sel_image = transform.widget_rect_to_image(sel_widget);

                let selected = if let Some(ocr) = &state.ocr {
                    ocr.words
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, word)| word.bbox.intersects(&sel_image).then_some(idx))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                state.selected_indices = selected;
                needs_redraw = true;
            }
        }

        drop(state);
        if needs_redraw {
            self.queue_draw();
        }
    }

    fn on_drag_end(&self) {
        let mut state = self.imp().state.borrow_mut();
        state.selecting = false;
        state.panning = false;
        drop(state);
        self.update_cursor();
        self.queue_draw();
    }

    fn on_pinch_begin(&self, gesture: &gtk::GestureZoom) {
        let mut state = self.imp().state.borrow_mut();
        if state.image_width <= 0.0 || state.image_height <= 0.0 {
            return;
        }

        let default_anchor = self.widget_center();
        let anchor = gesture
            .bounding_box_center()
            .map(|(x, y)| Point { x, y })
            .unwrap_or(default_anchor);

        state.pinch_active = true;
        state.pinch_start_zoom_factor = state.zoom_factor;
        state.pinch_start_center_img = state.center_img;
        state.pinch_anchor_widget = anchor;
    }

    fn on_pinch_scale_changed(&self, gesture: &gtk::GestureZoom, scale_factor: f64) {
        let mut state = self.imp().state.borrow_mut();
        if !state.pinch_active || state.image_width <= 0.0 || state.image_height <= 0.0 {
            return;
        }

        let widget_w = self.width() as f64;
        let widget_h = self.height() as f64;
        if widget_w <= 0.0 || widget_h <= 0.0 {
            return;
        }

        let begin_transform = {
            let begin_scale = ViewTransform::from_center(
                widget_w,
                widget_h,
                state.image_width,
                state.image_height,
                state.pinch_start_zoom_factor,
                state.pinch_start_center_img,
            )
            .scale;
            let begin_center = ViewTransform::clamp_center(
                widget_w,
                widget_h,
                state.image_width,
                state.image_height,
                begin_scale,
                state.pinch_start_center_img,
            );
            ViewTransform::from_center(
                widget_w,
                widget_h,
                state.image_width,
                state.image_height,
                state.pinch_start_zoom_factor,
                begin_center,
            )
        };

        let fallback_anchor = if state.pinch_anchor_widget == Point::default() {
            self.widget_center()
        } else {
            state.pinch_anchor_widget
        };
        let anchor_widget = gesture
            .bounding_box_center()
            .map(|(x, y)| Point { x, y })
            .unwrap_or(fallback_anchor);
        state.pinch_anchor_widget = anchor_widget;
        let anchor_img = begin_transform.widget_to_image(anchor_widget);
        let gesture_scale = if scale_factor > 0.0 {
            scale_factor
        } else {
            gesture.scale_delta().max(f64::MIN_POSITIVE)
        };

        state.zoom_factor = clamp_zoom_factor(
            state.pinch_start_zoom_factor * gesture_scale,
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
        );

        let new_scale = ViewTransform::from_center(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            state.zoom_factor,
            state.center_img,
        )
        .scale;
        let (_, widget_center) =
            ViewTransform::contain(widget_w, widget_h, state.image_width, state.image_height);
        state.center_img = recenter_for_anchor(widget_center, new_scale, anchor_widget, anchor_img);
        state.center_img = ViewTransform::clamp_center(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            new_scale,
            state.center_img,
        );

        drop(state);
        self.queue_draw();
        self.update_cursor();
    }

    fn on_pinch_end(&self) {
        let mut state = self.imp().state.borrow_mut();
        state.pinch_active = false;
        drop(state);
        self.update_cursor();
    }

    fn zoom_at(&self, anchor_widget: Point, factor: f64) {
        if factor <= 0.0 {
            return;
        }

        let mut state = self.imp().state.borrow_mut();
        if state.image_width <= 0.0 || state.image_height <= 0.0 {
            return;
        }

        let widget_w = self.width() as f64;
        let widget_h = self.height() as f64;
        let Some(current_transform) = transform_for_widget(&mut state, widget_w, widget_h) else {
            return;
        };

        let anchor_img = current_transform.widget_to_image(anchor_widget);
        let new_zoom = clamp_zoom_factor(
            state.zoom_factor * factor,
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
        );
        if (new_zoom - state.zoom_factor).abs() <= f64::EPSILON {
            return;
        }
        state.zoom_factor = new_zoom;

        let new_scale = ViewTransform::from_center(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            state.zoom_factor,
            state.center_img,
        )
        .scale;
        let (_, widget_center) =
            ViewTransform::contain(widget_w, widget_h, state.image_width, state.image_height);
        state.center_img = recenter_for_anchor(widget_center, new_scale, anchor_widget, anchor_img);
        state.center_img = ViewTransform::clamp_center(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            new_scale,
            state.center_img,
        );

        drop(state);
        self.queue_draw();
        self.update_cursor();
    }

    fn widget_center(&self) -> Point {
        Point {
            x: (self.width() as f64) * 0.5,
            y: (self.height() as f64) * 0.5,
        }
    }

    fn last_cursor_widget(&self) -> Point {
        self.imp()
            .state
            .borrow()
            .last_cursor_widget
            .unwrap_or_else(|| self.widget_center())
    }

    fn update_cursor(&self) {
        let state = self.imp().state.borrow();
        let can_pan = can_pan_at_view(
            self.width() as f64,
            self.height() as f64,
            state.image_width,
            state.image_height,
            state.zoom_factor,
        );
        if state.panning && can_pan {
            self.set_cursor_from_name(Some("grabbing"));
        } else if can_pan {
            self.set_cursor_from_name(Some("grab"));
        } else {
            self.set_cursor_from_name(None);
        }
    }
}

fn transform_for_widget(
    state: &mut imp::CanvasState,
    widget_w: f64,
    widget_h: f64,
) -> Option<ViewTransform> {
    if widget_w <= 0.0 || widget_h <= 0.0 || state.image_width <= 0.0 || state.image_height <= 0.0 {
        return None;
    }

    let max_zoom =
        max_zoom_factor_for_dims(widget_w, widget_h, state.image_width, state.image_height);
    if state.zoom_factor > max_zoom {
        state.zoom_factor = max_zoom;
    } else if state.zoom_factor < MIN_ZOOM_FACTOR {
        state.zoom_factor = MIN_ZOOM_FACTOR;
    }

    let mut transform = ViewTransform::from_center(
        widget_w,
        widget_h,
        state.image_width,
        state.image_height,
        state.zoom_factor,
        state.center_img,
    );
    let clamped = ViewTransform::clamp_center(
        widget_w,
        widget_h,
        state.image_width,
        state.image_height,
        transform.scale,
        state.center_img,
    );
    if point_changed(clamped, state.center_img) {
        state.center_img = clamped;
        transform = ViewTransform::from_center(
            widget_w,
            widget_h,
            state.image_width,
            state.image_height,
            state.zoom_factor,
            state.center_img,
        );
    }

    Some(transform)
}

fn point_changed(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() > f64::EPSILON || (a.y - b.y).abs() > f64::EPSILON
}

fn natural_size_for_measure(
    orientation: gtk::Orientation,
    for_size: i32,
    image_w: f64,
    image_h: f64,
) -> i32 {
    if image_w <= 0.0 || image_h <= 0.0 {
        return 1;
    }

    if for_size > 0 {
        match orientation {
            gtk::Orientation::Horizontal => size_to_i32((for_size as f64) * (image_w / image_h)),
            gtk::Orientation::Vertical => size_to_i32((for_size as f64) * (image_h / image_w)),
            _ => 1,
        }
    } else {
        match orientation {
            gtk::Orientation::Horizontal => size_to_i32(image_w),
            gtk::Orientation::Vertical => size_to_i32(image_h),
            _ => 1,
        }
    }
}

fn size_to_i32(value: f64) -> i32 {
    value.round().clamp(1.0, i32::MAX as f64) as i32
}

fn clamp_zoom_factor(zoom: f64, widget_w: f64, widget_h: f64, image_w: f64, image_h: f64) -> f64 {
    let max_zoom = max_zoom_factor_for_dims(widget_w, widget_h, image_w, image_h);
    zoom.clamp(MIN_ZOOM_FACTOR, max_zoom)
}

fn contain_scale_for_dims(widget_w: f64, widget_h: f64, image_w: f64, image_h: f64) -> Option<f64> {
    if widget_w <= 0.0 || widget_h <= 0.0 || image_w <= 0.0 || image_h <= 0.0 {
        return None;
    }
    Some(ViewTransform::contain(widget_w, widget_h, image_w, image_h).0)
}

fn effective_scale_for_dims(
    widget_w: f64,
    widget_h: f64,
    image_w: f64,
    image_h: f64,
    zoom_factor: f64,
) -> Option<f64> {
    contain_scale_for_dims(widget_w, widget_h, image_w, image_h).map(|s| s * zoom_factor)
}

fn max_zoom_factor_for_dims(widget_w: f64, widget_h: f64, image_w: f64, image_h: f64) -> f64 {
    let contain_scale =
        contain_scale_for_dims(widget_w, widget_h, image_w, image_h).unwrap_or(MIN_ZOOM_FACTOR);
    let max_zoom = BASE_MAX_ZOOM_FACTOR.max(1.0 / contain_scale);
    debug_assert!(contain_scale * max_zoom >= 1.0 - 1e-12);
    max_zoom
}

fn can_pan_at_view(
    widget_w: f64,
    widget_h: f64,
    image_w: f64,
    image_h: f64,
    zoom_factor: f64,
) -> bool {
    let Some(scale) = effective_scale_for_dims(widget_w, widget_h, image_w, image_h, zoom_factor)
    else {
        return false;
    };
    image_w * scale > widget_w + PAN_DIM_EPS || image_h * scale > widget_h + PAN_DIM_EPS
}

fn recenter_for_anchor(
    widget_center: Point,
    scale: f64,
    anchor_widget: Point,
    anchor_img: Point,
) -> Point {
    Point {
        x: anchor_img.x - (anchor_widget.x - widget_center.x) / scale,
        y: anchor_img.y - (anchor_widget.y - widget_center.y) / scale,
    }
}

fn scaling_filter_for_scale(scale: f64) -> gtk::gsk::ScalingFilter {
    let is_near_integer = scale > 1.0 && (scale - scale.round()).abs() <= INTEGER_SCALE_EPS;
    if is_near_integer {
        gtk::gsk::ScalingFilter::Nearest
    } else {
        gtk::gsk::ScalingFilter::Trilinear
    }
}

#[cfg(test)]
mod tests {
    use super::{max_zoom_factor_for_dims, natural_size_for_measure, size_to_i32};
    use gtk4::Orientation;
    use quickview_core::geometry::ViewTransform;

    #[test]
    fn size_to_i32_rounds_and_clamps() {
        assert_eq!(size_to_i32(-100.0), 1);
        assert_eq!(size_to_i32(0.0), 1);
        assert_eq!(size_to_i32(0.49), 1);
        assert_eq!(size_to_i32(0.50), 1);
        assert_eq!(size_to_i32(1.4), 1);
        assert_eq!(size_to_i32(1.5), 2);
        assert_eq!(size_to_i32((i32::MAX as f64) + 12345.0), i32::MAX);
    }

    #[test]
    fn measure_preserves_aspect_ratio_when_constrained() {
        // 2:1 image
        let image_w = 400.0;
        let image_h = 200.0;

        let h = natural_size_for_measure(Orientation::Horizontal, 100, image_w, image_h);
        assert_eq!(h, 200);

        let v = natural_size_for_measure(Orientation::Vertical, 100, image_w, image_h);
        assert_eq!(v, 50);
    }

    #[test]
    fn measure_uses_image_dimensions_when_unconstrained() {
        let image_w = 123.2;
        let image_h = 456.6;

        let h = natural_size_for_measure(Orientation::Horizontal, -1, image_w, image_h);
        assert_eq!(h, 123);

        let v = natural_size_for_measure(Orientation::Vertical, -1, image_w, image_h);
        assert_eq!(v, 457);
    }

    #[test]
    fn dynamic_max_zoom_allows_absolute_scale_one_for_tiny_contain() {
        let widget_w = 320.0;
        let widget_h = 240.0;
        let image_w = 12000.0;
        let image_h = 8000.0;

        let contain_scale = ViewTransform::contain(widget_w, widget_h, image_w, image_h).0;
        let max_zoom = max_zoom_factor_for_dims(widget_w, widget_h, image_w, image_h);
        let max_absolute_scale = contain_scale * max_zoom;

        assert!(max_zoom > 20.0);
        assert!(max_absolute_scale >= 1.0);
    }
}
