use serde::{Deserialize, Serialize};

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
