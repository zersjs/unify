use unify_proto::RATIO_SCALE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn fit_rect(stream: (u32, u32), client: (u32, u32)) -> FitRect {
    let (sw, sh) = (i64::from(stream.0.max(1)), i64::from(stream.1.max(1)));
    let (cw, ch) = (i64::from(client.0.max(1)), i64::from(client.1.max(1)));
    let (width, height) = if cw * sh <= ch * sw { (cw, cw * sh / sw) } else { (ch * sw / sh, ch) };
    FitRect { left: ((cw - width) / 2) as i32, top: ((ch - height) / 2) as i32, width: width as i32, height: height as i32 }
}

pub fn pointer_ratio(fit: FitRect, x: i32, y: i32) -> (u16, u16) {
    let scale = |value: i32, start: i32, span: i32| {
        let offset = i64::from((value - start).clamp(0, (span - 1).max(0)));
        ((offset * i64::from(RATIO_SCALE)) / i64::from((span - 1).max(1))) as u16
    };
    (scale(x, fit.left, fit.width), scale(y, fit.top, fit.height))
}

pub fn stream_size(pixels: (u32, u32), target: (u32, u32)) -> (u32, u32) {
    let scale = (f64::from(target.0) / f64::from(pixels.0.max(1))).min(f64::from(target.1) / f64::from(pixels.1.max(1))).min(1.0);
    let even = |value: u32| ((f64::from(value) * scale).round() as u32 & !1).max(2);
    (even(pixels.0), even(pixels.1))
}

pub fn pick_display_mode(modes: &[(u32, u32)], target: (u32, u32)) -> Option<usize> {
    let aspect = |size: (u32, u32)| f64::from(size.0) / f64::from(size.1.max(1));
    let wanted = aspect(target);
    let key = |index: usize| {
        let mode = modes[index];
        let aspect_error = ((aspect(mode) - wanted).abs() * 1000.0).round() as u64;
        let too_small = u64::from(mode.0 < target.0);
        (aspect_error, too_small, u64::from(mode.0.abs_diff(target.0)))
    };
    (0..modes.len()).min_by_key(|index| key(*index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_size_never_upscales_and_stays_even() {
        assert_eq!(stream_size((2880, 1864), (1920, 1080)), (1668, 1080));
        assert_eq!(stream_size((1280, 720), (1920, 1080)), (1280, 720));
        assert_eq!(stream_size((1367, 769), (1367, 769)), (1366, 768));
    }

    #[test]
    fn picks_matching_aspect_then_closest_size_at_or_above_target() {
        let modes = [(2880, 1864), (2560, 1440), (1920, 1080), (1600, 900), (1470, 956)];
        assert_eq!(pick_display_mode(&modes, (1920, 1080)), Some(2));
        assert_eq!(pick_display_mode(&modes, (2000, 1125)), Some(1));
        assert_eq!(pick_display_mode(&modes, (1920, 1200)), Some(0));
        assert_eq!(pick_display_mode(&[], (1920, 1080)), None);
    }

    #[test]
    fn same_aspect_fills_the_window() {
        assert_eq!(fit_rect((2560, 1600), (1280, 800)), FitRect { left: 0, top: 0, width: 1280, height: 800 });
    }

    #[test]
    fn narrower_stream_is_pillarboxed() {
        assert_eq!(fit_rect((1600, 1000), (1920, 1080)), FitRect { left: 96, top: 0, width: 1728, height: 1080 });
    }

    #[test]
    fn wider_stream_is_letterboxed() {
        assert_eq!(fit_rect((1920, 1080), (1000, 1000)), FitRect { left: 0, top: 219, width: 1000, height: 562 });
    }

    #[test]
    fn pointer_maps_to_stream_corners_and_clamps_outside() {
        let fit = FitRect { left: 96, top: 0, width: 1728, height: 1080 };
        assert_eq!(pointer_ratio(fit, 96, 0), (0, 0));
        assert_eq!(pointer_ratio(fit, 96 + 1727, 1079), (RATIO_SCALE, RATIO_SCALE));
        assert_eq!(pointer_ratio(fit, 0, 5000), (0, RATIO_SCALE));
    }
}
