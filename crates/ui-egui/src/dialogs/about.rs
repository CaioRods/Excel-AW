use crate::{i18n::Language, theme};

const WILSON_LOGO: &[u8] = include_bytes!("../../../../assets/wilson/logo.png");

pub(super) fn about(ui: &mut egui::Ui, lang: Language) {
    let tokens = theme::Tokens::get(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        let id = egui::Id::unique("excelaw-wilson-logo");
        let mut texture = ui.ctx().data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
        if texture.is_none()
            && let Ok(image) = image::load_from_memory_with_format(WILSON_LOGO, image::ImageFormat::Png)
        {
            let rgba = image.into_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
            let logo = ui.ctx().load_texture("Alimentos Wilson", pixels, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|data| data.insert_temp(id, logo.clone()));
            texture = Some(logo);
        }
        if let Some(texture) = texture {
            let size = texture.size_vec2() * (156.0 / texture.size_vec2().x);
            ui.add(egui::Image::new((texture.id(), size)).alt_text("Alimentos Wilson"));
            ui.add_space(12.0);
        }
        ui.label(egui::RichText::new("Excel AW").font(theme::ui_bold(26.0)));
        ui.label(egui::RichText::new(lang.tr("Version {version}").replace("{version}", env!("CARGO_PKG_VERSION"))).color(tokens.text_dim));
        ui.add_space(12.0);
        ui.label(lang.tr("Edition author"));
        ui.label(egui::RichText::new("Caio Victor Rodrigues").strong());
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Icarus 4.5").font(theme::ui_bold(15.0)).color(tokens.accent));
        ui.hyperlink_to(lang.tr("Licenses and attribution"), "https://github.com/CaioRods/Excel-AW/blob/feat/excelaw-pt-br/NOTICE");
    });
}
