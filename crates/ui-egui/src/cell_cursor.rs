//! Original white cell-selection cross, uploaded as an OS cursor so it follows the mouse
//! independently of document repainting. Unsupported integrations keep the standard Cell cursor.

pub(crate) fn set(ctx: &egui::Context) {
    let scale = ctx.native_pixels_per_point().filter(|value| value.is_finite() && *value > 0.0).unwrap_or(1.0).ceil().clamp(1.0, 4.0) as u16;
    let id = egui::Id::unique("aw_cell_cursor_bitmap").with(scale);
    let image = ctx.data_mut(|data| data.get_temp::<egui::CustomCursorImage>(id)).unwrap_or_else(|| {
        let image = bitmap(scale);
        ctx.data_mut(|data| data.insert_temp(id, image.clone()));
        image
    });
    ctx.set_cursor_icon(egui::CursorIcon::Cell);
    ctx.set_cursor_image(Some(image));
}

fn bitmap(scale: u16) -> egui::CustomCursorImage {
    let scale = scale.clamp(1, 4);
    let side = 21 * scale;
    let mut rgba = Vec::with_capacity(usize::from(side) * usize::from(side) * 4);
    for y in 0..side {
        for x in 0..side {
            let dx = (x / scale).abs_diff(10);
            let dy = (y / scale).abs_diff(10);
            let outer = (dx <= 3 && dy <= 9) || (dy <= 3 && dx <= 9);
            let inner = (dx <= 2 && dy <= 8) || (dy <= 2 && dx <= 8);
            rgba.extend_from_slice(if inner {
                &[255, 255, 255, 255]
            } else if outer {
                &[0, 0, 0, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
    }
    egui::CustomCursorImage { rgba: rgba.into(), size: [side, side], hotspot: [10 * scale, 10 * scale] }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bitmap_has_white_fill_black_outline_transparency_and_scaled_hotspot() {
        for scale in 1..=4 {
            let image = super::bitmap(scale);
            assert_eq!(image.rgba.len(), usize::from(image.size[0]) * usize::from(image.size[1]) * 4);
            assert_eq!(image.hotspot, [10 * scale, 10 * scale]);
            for color in [[255, 255, 255, 255], [0, 0, 0, 255], [0, 0, 0, 0]] {
                assert!(image.rgba.as_chunks::<4>().0.contains(&color));
            }
        }
    }

    #[test]
    fn native_cursor_is_cached_and_standard_cursor_remains_available_as_fallback() {
        let ctx = egui::Context::default();
        let mut first = ctx.run_ui(egui::RawInput::default(), |ui| super::set(ui.ctx()));
        assert_eq!(first.platform_output.cursor_icon, egui::CursorIcon::Cell);
        let image = first.platform_output.cursor_image.take().unwrap();
        first.drop_without_applying_deltas();
        let mut second = ctx.run_ui(egui::RawInput::default(), |ui| super::set(ui.ctx()));
        let same = second.platform_output.cursor_image.take().unwrap();
        assert!(std::sync::Arc::ptr_eq(&image.rgba, &same.rgba), "do not upload a new cursor every pointer frame");
        second.drop_without_applying_deltas();
    }
}
