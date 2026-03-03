use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn from_points(a: Point, b: Point) -> Self {
        let x1 = a.x.min(b.x);
        let y1 = a.y.min(b.y);
        let x2 = a.x.max(b.x);
        let y2 = a.y.max(b.y);
        Rect {
            x: x1,
            y: y1,
            w: (x2 - x1).max(0.0),
            h: (y2 - y1).max(0.0),
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        let ax2 = self.x + self.w;
        let ay2 = self.y + self.h;
        let bx2 = other.x + other.w;
        let by2 = other.y + other.h;

        self.x < bx2 && ax2 > other.x && self.y < by2 && ay2 > other.y
    }
}

/// Result of `ViewTransform::contain()`.
///
/// This represents the baseline "fit to widget" (contain) scale and the widget-space
/// center point used by `ViewTransform::from_center()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContainResult {
    /// Uniform scale that fits the entire image inside the widget.
    pub contain_scale: f64,

    /// Center of the widget in widget coordinates (pixels).
    pub widget_center: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTransformError {
    NonFinite,
    NonPositiveScale,
}

impl fmt::Display for ViewTransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewTransformError::NonFinite => write!(f, "non-finite view transform value"),
            ViewTransformError::NonPositiveScale => write!(f, "scale must be > 0"),
        }
    }
}

impl std::error::Error for ViewTransformError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewTransform {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

impl ViewTransform {
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Result<Self, ViewTransformError> {
        if !scale.is_finite() || !offset_x.is_finite() || !offset_y.is_finite() {
            return Err(ViewTransformError::NonFinite);
        }
        if scale <= 0.0 {
            return Err(ViewTransformError::NonPositiveScale);
        }
        Ok(Self {
            scale,
            offset_x,
            offset_y,
        })
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn offset_x(&self) -> f64 {
        self.offset_x
    }

    pub fn offset_y(&self) -> f64 {
        self.offset_y
    }

    /// Compute the baseline "contain" (fit-to-widget) scale.
    ///
    /// The returned `widget_center` is in widget coordinates (pixels) and is the point
    /// that `from_center()` treats as the widget's visual center anchor.
    pub fn contain(widget_w: f64, widget_h: f64, image_w: f64, image_h: f64) -> ContainResult {
        let widget_center = Point {
            x: widget_w.max(0.0) * 0.5,
            y: widget_h.max(0.0) * 0.5,
        };

        if widget_w <= 0.0 || widget_h <= 0.0 || image_w <= 0.0 || image_h <= 0.0 {
            return ContainResult {
                contain_scale: 1.0,
                widget_center,
            };
        }

        let contain_scale = (widget_w / image_w)
            .min(widget_h / image_h)
            .max(f64::MIN_POSITIVE);
        ContainResult {
            contain_scale,
            widget_center,
        }
    }

    /// Construct a `ViewTransform` from canonical view state.
    ///
    /// `center_img.x` and `center_img.y` must be finite. This function delegates validation
    /// to `ViewTransform::new` and will panic if invariants are violated.
    pub fn from_center(
        widget_w: f64,
        widget_h: f64,
        image_w: f64,
        image_h: f64,
        zoom_factor: f64,
        center_img: Point,
    ) -> Self {
        // `from_center()` delegates invariants to `ViewTransform::new`.
        // `center_img.x` / `center_img.y` must be finite or `ViewTransform::new` will error.
        debug_assert!(
            center_img.x.is_finite() && center_img.y.is_finite(),
            "from_center: center_img must be finite (x={}, y={})",
            center_img.x,
            center_img.y
        );

        let contain = Self::contain(widget_w, widget_h, image_w, image_h);
        let scale =
            (contain.contain_scale * zoom_factor.max(f64::MIN_POSITIVE)).max(f64::MIN_POSITIVE);

        let offset_x = contain.widget_center.x - center_img.x * scale;
        let offset_y = contain.widget_center.y - center_img.y * scale;
        Self::new(scale, offset_x, offset_y).expect("ViewTransform invariants violated")
    }

    pub fn image_to_widget(&self, point: Point) -> Point {
        Point {
            x: self.offset_x + point.x * self.scale,
            y: self.offset_y + point.y * self.scale,
        }
    }

    pub fn widget_to_image(&self, point: Point) -> Point {
        Point {
            x: (point.x - self.offset_x) / self.scale,
            y: (point.y - self.offset_y) / self.scale,
        }
    }

    pub fn image_rect_to_widget(&self, rect: Rect) -> Rect {
        Rect {
            x: self.offset_x + rect.x * self.scale,
            y: self.offset_y + rect.y * self.scale,
            w: rect.w * self.scale,
            h: rect.h * self.scale,
        }
    }

    pub fn widget_rect_to_image(&self, rect: Rect) -> Rect {
        Rect {
            x: (rect.x - self.offset_x) / self.scale,
            y: (rect.y - self.offset_y) / self.scale,
            w: rect.w / self.scale,
            h: rect.h / self.scale,
        }
    }

    pub fn clamp_center(
        widget_w: f64,
        widget_h: f64,
        image_w: f64,
        image_h: f64,
        scale: f64,
        center_img: Point,
    ) -> Point {
        if widget_w <= 0.0 || widget_h <= 0.0 || image_w <= 0.0 || image_h <= 0.0 || scale <= 0.0 {
            return center_img;
        }

        let half_view_w = widget_w / (2.0 * scale);
        let half_view_h = widget_h / (2.0 * scale);

        let center_x = if image_w * scale <= widget_w {
            image_w * 0.5
        } else {
            center_img.x.clamp(half_view_w, image_w - half_view_w)
        };

        let center_y = if image_h * scale <= widget_h {
            image_h * 0.5
        } else {
            center_img.y.clamp(half_view_h, image_h - half_view_h)
        };

        Point {
            x: center_x,
            y: center_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Point, ViewTransform};

    fn approx_eq(a: f64, b: f64, eps: f64) {
        assert!((a - b).abs() <= eps, "{a} != {b} (eps={eps})");
    }

    #[test]
    fn image_and_widget_mapping_are_inverse() {
        let t = ViewTransform::from_center(
            1200.0,
            800.0,
            2400.0,
            1600.0,
            2.25,
            Point { x: 900.0, y: 600.0 },
        );

        let img = Point {
            x: 1234.5,
            y: 345.25,
        };
        let widget = t.image_to_widget(img);
        let roundtrip = t.widget_to_image(widget);

        approx_eq(roundtrip.x, img.x, 1e-9);
        approx_eq(roundtrip.y, img.y, 1e-9);
    }

    #[test]
    fn anchor_preserving_zoom_keeps_widget_anchor_fixed() {
        let widget_w = 1000.0;
        let widget_h = 700.0;
        let image_w = 3000.0;
        let image_h = 2000.0;

        let center_start = Point {
            x: 1300.0,
            y: 900.0,
        };
        let zoom_start = 1.3;
        let zoom_new = 2.1;
        let anchor_widget = Point { x: 120.0, y: 520.0 };

        let t_start = ViewTransform::from_center(
            widget_w,
            widget_h,
            image_w,
            image_h,
            zoom_start,
            center_start,
        );
        let anchor_img = t_start.widget_to_image(anchor_widget);

        let t_new_unclamped = ViewTransform::from_center(
            widget_w,
            widget_h,
            image_w,
            image_h,
            zoom_new,
            center_start,
        );
        let contain = ViewTransform::contain(widget_w, widget_h, image_w, image_h);
        let widget_center = contain.widget_center;
        let center_new = Point {
            x: anchor_img.x - (anchor_widget.x - widget_center.x) / t_new_unclamped.scale(),
            y: anchor_img.y - (anchor_widget.y - widget_center.y) / t_new_unclamped.scale(),
        };
        let t_new =
            ViewTransform::from_center(widget_w, widget_h, image_w, image_h, zoom_new, center_new);

        let mapped_anchor = t_new.image_to_widget(anchor_img);
        approx_eq(mapped_anchor.x, anchor_widget.x, 1e-9);
        approx_eq(mapped_anchor.y, anchor_widget.y, 1e-9);
    }

    #[test]
    fn clamp_center_forces_image_center_when_scaled_image_fits() {
        let center =
            ViewTransform::clamp_center(1000.0, 800.0, 300.0, 200.0, 2.0, Point { x: 0.0, y: 0.0 });
        approx_eq(center.x, 150.0, 1e-9);
        approx_eq(center.y, 100.0, 1e-9);
    }

    #[test]
    fn clamp_center_limits_pan_when_scaled_image_exceeds_viewport() {
        let center = ViewTransform::clamp_center(
            1000.0,
            700.0,
            3000.0,
            2000.0,
            0.6,
            Point {
                x: -5000.0,
                y: 5000.0,
            },
        );

        // half_view_w = 1000 / (2 * 0.6) = 833.333...
        // half_view_h = 700 / (2 * 0.6) = 583.333...
        approx_eq(center.x, 833.3333333333334, 1e-9);
        approx_eq(center.y, 1416.6666666666667, 1e-9);
    }
}
