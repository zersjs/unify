use serde::{Deserialize, Serialize};
use unify_proto::{ratio_to_f64, EntryPoint, Side};

pub const ENTRY_INSET_PX: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    fn clamp_point(&self, px: i32, py: i32) -> (i32, i32) {
        (px.clamp(self.x, self.right() - 1), py.clamp(self.y, self.bottom() - 1))
    }

    fn distance_sq(&self, px: i32, py: i32) -> i64 {
        let (cx, cy) = self.clamp_point(px, py);
        let dx = i64::from(px - cx);
        let dy = i64::from(py - cy);
        dx * dx + dy * dy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Desktop {
    displays: Vec<Rect>,
}

impl Desktop {
    pub fn new(displays: Vec<Rect>) -> Option<Self> {
        let displays: Vec<Rect> = displays.into_iter().filter(|d| d.width > 0 && d.height > 0).collect();
        if displays.is_empty() {
            return None;
        }
        Some(Self { displays })
    }

    pub fn displays(&self) -> &[Rect] {
        &self.displays
    }

    pub fn primary(&self) -> Rect {
        self.displays.iter().copied().find(|d| d.contains(0, 0)).unwrap_or(self.displays[0])
    }

    pub fn center(&self) -> (i32, i32) {
        let primary = self.primary();
        (primary.x + primary.width / 2, primary.y + primary.height / 2)
    }

    pub fn bounds(&self) -> Rect {
        let left = self.displays.iter().map(|d| d.x).min().unwrap_or(0);
        let top = self.displays.iter().map(|d| d.y).min().unwrap_or(0);
        let right = self.displays.iter().map(Rect::right).max().unwrap_or(1);
        let bottom = self.displays.iter().map(Rect::bottom).max().unwrap_or(1);
        Rect::new(left, top, right - left, bottom - top)
    }

    pub fn display_at(&self, px: i32, py: i32) -> Option<Rect> {
        self.displays.iter().copied().find(|d| d.contains(px, py))
    }

    fn nearest_display(&self, px: i32, py: i32) -> Rect {
        self.displays.iter().copied().min_by_key(|d| d.distance_sq(px, py)).unwrap_or(self.displays[0])
    }

    pub fn clamp(&self, px: i32, py: i32) -> (i32, i32) {
        if self.display_at(px, py).is_some() {
            return (px, py);
        }
        self.nearest_display(px, py).clamp_point(px, py)
    }

    pub fn at_edge(&self, px: i32, py: i32, side: Side) -> bool {
        let (cx, cy) = self.clamp(px, py);
        let display = self.nearest_display(cx, cy);
        match side {
            Side::Right => cx >= display.right() - 1 && self.display_at(cx + 1, cy).is_none(),
            Side::Left => cx <= display.x && self.display_at(cx - 1, cy).is_none(),
            Side::Top => cy <= display.y && self.display_at(cx, cy - 1).is_none(),
            Side::Bottom => cy >= display.bottom() - 1 && self.display_at(cx, cy + 1).is_none(),
        }
    }

    pub fn in_corner(&self, px: i32, py: i32, side: Side, guard: i32) -> bool {
        if guard <= 0 {
            return false;
        }
        let (cx, cy) = self.clamp(px, py);
        let display = self.nearest_display(cx, cy);
        if side.is_horizontal() {
            cy < display.y + guard || cy >= display.bottom() - guard
        } else {
            cx < display.x + guard || cx >= display.right() - guard
        }
    }

    pub fn ratio_along(&self, px: i32, py: i32, side: Side) -> f64 {
        let bounds = self.bounds();
        let (cx, cy) = self.clamp(px, py);
        if side.is_horizontal() {
            let span = (bounds.height - 1).max(1);
            f64::from(cy - bounds.y) / f64::from(span)
        } else {
            let span = (bounds.width - 1).max(1);
            f64::from(cx - bounds.x) / f64::from(span)
        }
    }

    pub fn entry_point(&self, entry: EntryPoint) -> (i32, i32) {
        let (side, ratio) = match entry {
            EntryPoint::Center => return self.center(),
            EntryPoint::Edge { side, ratio } => (side, ratio_to_f64(ratio)),
        };
        let bounds = self.bounds();
        if side.is_horizontal() {
            let y = bounds.y + (ratio * f64::from(bounds.height - 1)).round() as i32;
            let candidates = self.displays.iter().copied().filter(|d| y >= d.y && y < d.bottom());
            let chosen = match side {
                Side::Left => candidates.min_by_key(|d| d.x),
                _ => candidates.max_by_key(Rect::right),
            };
            let display = chosen.unwrap_or_else(|| self.nearest_display(bounds.x, y));
            let x = match side {
                Side::Left => display.x + ENTRY_INSET_PX,
                _ => display.right() - 1 - ENTRY_INSET_PX,
            };
            display.clamp_point(x, y)
        } else {
            let x = bounds.x + (ratio * f64::from(bounds.width - 1)).round() as i32;
            let candidates = self.displays.iter().copied().filter(|d| x >= d.x && x < d.right());
            let chosen = match side {
                Side::Top => candidates.min_by_key(|d| d.y),
                _ => candidates.max_by_key(Rect::bottom),
            };
            let display = chosen.unwrap_or_else(|| self.nearest_display(x, bounds.y));
            let y = match side {
                Side::Top => display.y + ENTRY_INSET_PX,
                _ => display.bottom() - 1 - ENTRY_INSET_PX,
            };
            display.clamp_point(x, y)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unify_proto::{ratio_from_f64, RATIO_SCALE};

    fn single() -> Desktop {
        Desktop::new(vec![Rect::new(0, 0, 1920, 1080)]).unwrap()
    }

    fn dual_offset() -> Desktop {
        Desktop::new(vec![Rect::new(0, 0, 1512, 982), Rect::new(1512, -200, 2560, 1440)]).unwrap()
    }

    #[test]
    fn rejects_empty_and_degenerate_layouts() {
        assert!(Desktop::new(vec![]).is_none());
        assert!(Desktop::new(vec![Rect::new(0, 0, 0, 100)]).is_none());
    }

    #[test]
    fn detects_outer_edges_only() {
        let desktop = dual_offset();
        assert!(!desktop.at_edge(1511, 500, Side::Right));
        assert!(desktop.at_edge(4071, 500, Side::Right));
        assert!(desktop.at_edge(5000, 500, Side::Right));
        assert!(desktop.at_edge(0, 500, Side::Left));
        assert!(!desktop.at_edge(1512, 500, Side::Left));
        assert!(desktop.at_edge(1600, -200, Side::Top));
        assert!(desktop.at_edge(100, 0, Side::Top));
        assert!(!desktop.at_edge(100, 1, Side::Top));
    }

    #[test]
    fn bottom_edge_of_shorter_display_counts_when_nothing_below() {
        let desktop = dual_offset();
        assert!(desktop.at_edge(500, 981, Side::Bottom));
        assert!(desktop.at_edge(2000, 1239, Side::Bottom));
    }

    #[test]
    fn clamps_points_into_nearest_display() {
        let desktop = dual_offset();
        assert_eq!(desktop.clamp(-50, 100), (0, 100));
        assert_eq!(desktop.clamp(800, 1200), (800, 981));
        assert_eq!(desktop.clamp(2000, 1500), (2000, 1239));
    }

    #[test]
    fn ratio_round_trips_through_entry_point() {
        let desktop = single();
        for y in [0, 1, 540, 1078, 1079] {
            let ratio = ratio_from_f64(desktop.ratio_along(1919, y, Side::Right));
            let (ex, ey) = desktop.entry_point(EntryPoint::Edge { side: Side::Right, ratio });
            assert_eq!(ex, 1919 - ENTRY_INSET_PX);
            assert!((ey - y).abs() <= 1, "y {y} -> {ey}");
        }
    }

    #[test]
    fn entry_point_is_inside_a_display_and_not_on_the_edge() {
        let desktop = dual_offset();
        for ratio in [0, 1, RATIO_SCALE / 3, RATIO_SCALE / 2, RATIO_SCALE] {
            for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
                let (x, y) = desktop.entry_point(EntryPoint::Edge { side, ratio });
                assert!(desktop.display_at(x, y).is_some(), "{side:?} {ratio} -> ({x},{y})");
            }
        }
        let (x, y) = desktop.entry_point(EntryPoint::Edge { side: Side::Left, ratio: 5000 });
        assert!(!desktop.at_edge(x, y, Side::Left));
    }

    #[test]
    fn center_uses_primary_display() {
        let desktop = Desktop::new(vec![Rect::new(-1920, 0, 1920, 1080), Rect::new(0, 0, 1000, 800)]).unwrap();
        assert_eq!(desktop.center(), (500, 400));
    }

    #[test]
    fn corner_guard() {
        let desktop = single();
        assert!(desktop.in_corner(1919, 5, Side::Right, 20));
        assert!(desktop.in_corner(1919, 1075, Side::Right, 20));
        assert!(!desktop.in_corner(1919, 500, Side::Right, 20));
        assert!(!desktop.in_corner(1919, 5, Side::Right, 0));
        assert!(desktop.in_corner(3, 0, Side::Top, 10));
    }
}
