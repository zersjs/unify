use unify_proto::{ratio_from_f64, EntryPoint, Side};

use crate::geometry::Desktop;

pub const RETURN_COOLDOWN_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeSwitchConfig {
    pub side: Side,
    pub enabled: bool,
    pub dwell_ms: u64,
    pub corner_guard_px: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDecision {
    Stay,
    Switch { ratio: u16 },
}

#[derive(Debug)]
pub struct EdgeSwitcher {
    config: EdgeSwitchConfig,
    armed: bool,
    edge_since: Option<u64>,
    cooldown_until: u64,
}

impl EdgeSwitcher {
    pub fn new(config: EdgeSwitchConfig) -> Self {
        Self { config, armed: false, edge_since: None, cooldown_until: 0 }
    }

    pub fn config(&self) -> EdgeSwitchConfig {
        self.config
    }

    pub fn reconfigure(&mut self, config: EdgeSwitchConfig) {
        if config != self.config {
            self.config = config;
            self.armed = false;
            self.edge_since = None;
        }
    }

    pub fn on_motion(&mut self, desktop: &Desktop, x: i32, y: i32, now_ms: u64, can_switch: bool) -> EdgeDecision {
        if !self.config.enabled {
            return EdgeDecision::Stay;
        }
        let side = self.config.side;
        let at_edge = desktop.at_edge(x, y, side) && !desktop.in_corner(x, y, side, self.config.corner_guard_px);
        if !at_edge {
            self.armed = true;
            self.edge_since = None;
            return EdgeDecision::Stay;
        }
        if !self.armed || now_ms < self.cooldown_until || !can_switch {
            self.edge_since = None;
            return EdgeDecision::Stay;
        }
        let since = *self.edge_since.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < self.config.dwell_ms {
            return EdgeDecision::Stay;
        }
        self.edge_since = None;
        self.armed = false;
        EdgeDecision::Switch { ratio: ratio_from_f64(desktop.ratio_along(x, y, side)) }
    }

    pub fn on_returned(&mut self, now_ms: u64) {
        self.armed = false;
        self.edge_since = None;
        self.cooldown_until = now_ms + RETURN_COOLDOWN_MS;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStep {
    Move { x: i32, y: i32 },
    Leave { side: Side, ratio: u16 },
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteCursor {
    return_side: Side,
    position: (i32, i32),
}

impl RemoteCursor {
    pub fn enter(desktop: &Desktop, entry: EntryPoint, configured_return_side: Side) -> Self {
        let return_side = match entry {
            EntryPoint::Edge { side, .. } => side,
            EntryPoint::Center => configured_return_side,
        };
        Self { return_side, position: desktop.entry_point(entry) }
    }

    pub fn position(&self) -> (i32, i32) {
        self.position
    }

    pub fn return_side(&self) -> Side {
        self.return_side
    }

    pub fn step(&mut self, desktop: &Desktop, actual: Option<(i32, i32)>, dx: i32, dy: i32) -> CursorStep {
        let (bx, by) = desktop.clamp(actual.map_or(self.position.0, |p| p.0), actual.map_or(self.position.1, |p| p.1));
        let tx = bx.saturating_add(dx);
        let ty = by.saturating_add(dy);
        let (cx, cy) = desktop.clamp(tx, ty);
        let outside = desktop.display_at(tx, ty).is_none();
        let side = self.return_side;
        let pushing = match side {
            Side::Left => tx < cx,
            Side::Right => tx > cx,
            Side::Top => ty < cy,
            Side::Bottom => ty > cy,
        };
        if outside && pushing && desktop.at_edge(cx, cy, side) {
            self.position = (cx, cy);
            return CursorStep::Leave { side, ratio: ratio_from_f64(desktop.ratio_along(cx, cy, side)) };
        }
        self.position = (cx, cy);
        CursorStep::Move { x: cx, y: cy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, ENTRY_INSET_PX};

    fn desktop() -> Desktop {
        Desktop::new(vec![Rect::new(0, 0, 1920, 1080)]).unwrap()
    }

    fn config(dwell_ms: u64, corner_guard_px: i32) -> EdgeSwitchConfig {
        EdgeSwitchConfig { side: Side::Right, enabled: true, dwell_ms, corner_guard_px }
    }

    #[test]
    fn switches_when_cursor_reaches_configured_edge_after_arming() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(0, 0));
        assert_eq!(s.on_motion(&d, 1919, 540, 0, true), EdgeDecision::Stay);
        assert_eq!(s.on_motion(&d, 1900, 540, 1, true), EdgeDecision::Stay);
        assert!(matches!(s.on_motion(&d, 1919, 540, 2, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn does_not_switch_on_other_edges() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(0, 0));
        s.on_motion(&d, 500, 500, 0, true);
        assert_eq!(s.on_motion(&d, 0, 500, 1, true), EdgeDecision::Stay);
        assert_eq!(s.on_motion(&d, 500, 0, 2, true), EdgeDecision::Stay);
    }

    #[test]
    fn does_not_switch_while_blocked_and_switches_once_unblocked() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(0, 0));
        s.on_motion(&d, 1000, 500, 0, true);
        assert_eq!(s.on_motion(&d, 1919, 500, 1, false), EdgeDecision::Stay);
        assert!(matches!(s.on_motion(&d, 1919, 501, 2, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn requires_rearm_and_cooldown_after_return() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(0, 0));
        s.on_motion(&d, 1000, 500, 0, true);
        assert!(matches!(s.on_motion(&d, 1919, 500, 1, true), EdgeDecision::Switch { .. }));
        s.on_returned(100);
        assert_eq!(s.on_motion(&d, 1919, 500, 110, true), EdgeDecision::Stay);
        assert_eq!(s.on_motion(&d, 1915, 500, 120, true), EdgeDecision::Stay);
        assert_eq!(s.on_motion(&d, 1919, 500, 130, true), EdgeDecision::Stay);
        s.on_motion(&d, 1915, 500, 400, true);
        assert!(matches!(s.on_motion(&d, 1919, 500, 401, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn dwell_delays_switch_until_cursor_stays_at_edge() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(200, 0));
        s.on_motion(&d, 1000, 500, 0, true);
        assert_eq!(s.on_motion(&d, 1919, 500, 10, true), EdgeDecision::Stay);
        assert_eq!(s.on_motion(&d, 1919, 501, 150, true), EdgeDecision::Stay);
        assert!(matches!(s.on_motion(&d, 1919, 502, 210, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn leaving_edge_resets_dwell_timer() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(200, 0));
        s.on_motion(&d, 1000, 500, 0, true);
        s.on_motion(&d, 1919, 500, 10, true);
        s.on_motion(&d, 1900, 500, 100, true);
        assert_eq!(s.on_motion(&d, 1919, 500, 220, true), EdgeDecision::Stay);
        assert!(matches!(s.on_motion(&d, 1919, 500, 430, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn corner_guard_blocks_switch_near_corners() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(config(0, 30));
        s.on_motion(&d, 1000, 500, 0, true);
        assert_eq!(s.on_motion(&d, 1919, 10, 1, true), EdgeDecision::Stay);
        assert!(matches!(s.on_motion(&d, 1919, 300, 2, true), EdgeDecision::Switch { .. }));
    }

    #[test]
    fn disabled_switcher_never_switches() {
        let d = desktop();
        let mut s = EdgeSwitcher::new(EdgeSwitchConfig { enabled: false, ..config(0, 0) });
        s.on_motion(&d, 1000, 500, 0, true);
        assert_eq!(s.on_motion(&d, 1919, 500, 1, true), EdgeDecision::Stay);
    }

    #[test]
    fn remote_cursor_enters_at_mirrored_position_and_leaves_through_return_edge() {
        let d = desktop();
        let mut cursor = RemoteCursor::enter(&d, EntryPoint::Edge { side: Side::Left, ratio: 5000 }, Side::Left);
        assert_eq!(cursor.position(), (ENTRY_INSET_PX, 540));
        assert_eq!(cursor.step(&d, None, 100, 0), CursorStep::Move { x: 102, y: 540 });
        assert_eq!(cursor.step(&d, None, 2000, 0), CursorStep::Move { x: 1919, y: 540 });
        assert_eq!(cursor.step(&d, None, 0, -5000), CursorStep::Move { x: 1919, y: 0 });
        assert_eq!(cursor.step(&d, None, -1919, 540), CursorStep::Move { x: 0, y: 540 });
        assert!(matches!(cursor.step(&d, None, -1, 0), CursorStep::Leave { side: Side::Left, .. }));
    }

    #[test]
    fn remote_cursor_does_not_leave_through_non_return_edges() {
        let d = desktop();
        let mut cursor = RemoteCursor::enter(&d, EntryPoint::Edge { side: Side::Left, ratio: 0 }, Side::Left);
        assert_eq!(cursor.step(&d, Some((1919, 10)), 50, 0), CursorStep::Move { x: 1919, y: 10 });
        assert_eq!(cursor.step(&d, Some((10, 0)), 0, -50), CursorStep::Move { x: 10, y: 0 });
    }

    #[test]
    fn remote_cursor_follows_actual_position_when_local_mouse_moved() {
        let d = desktop();
        let mut cursor = RemoteCursor::enter(&d, EntryPoint::Center, Side::Right);
        assert_eq!(cursor.return_side(), Side::Right);
        assert_eq!(cursor.step(&d, Some((100, 100)), 5, 5), CursorStep::Move { x: 105, y: 105 });
    }

    #[test]
    fn leave_ratio_matches_vertical_position() {
        let d = desktop();
        let mut cursor = RemoteCursor::enter(&d, EntryPoint::Edge { side: Side::Right, ratio: 0 }, Side::Right);
        match cursor.step(&d, Some((1919, 1079)), 3, 0) {
            CursorStep::Leave { ratio, .. } => assert_eq!(ratio, unify_proto::RATIO_SCALE),
            other => panic!("unexpected {other:?}"),
        }
    }
}
