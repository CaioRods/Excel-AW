use crate::{i18n::Language, theme};

const APP_LOGO: &[u8] = include_bytes!("../../../../assets/app-icon/excel-aw.png");

pub(super) fn about(ui: &mut egui::Ui, lang: Language) {
    let tokens = theme::Tokens::get(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        let id = egui::Id::unique("excelaw-about-logo");
        let mut texture = ui.ctx().data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
        if texture.is_none()
            && let Ok(image) = image::load_from_memory_with_format(APP_LOGO, image::ImageFormat::Png)
        {
            let rgba = image.into_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
            let logo = ui.ctx().load_texture("Excel AW", pixels, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|data| data.insert_temp(id, logo.clone()));
            texture = Some(logo);
        }
        let title_width = ui.painter().layout_no_wrap("Excel AW".into(), theme::ui_bold(26.0), tokens.text).size().x;
        let gap = if texture.is_some() { 68.0 } else { 0.0 };
        let left = ((ui.available_width() - title_width - gap) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(left);
            if let Some(texture) = texture {
                ui.add(egui::Image::new((texture.id(), egui::vec2(56.0, 56.0))).alt_text(if lang == Language::PtBr {
                    "Ícone do Excel AW"
                } else {
                    "Excel AW icon"
                }));
                ui.add_space(4.0);
            }
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Excel AW").font(theme::ui_bold(26.0)));
                ui.label(egui::RichText::new(lang.tr("Version {version}").replace("{version}", env!("CARGO_PKG_VERSION"))).color(tokens.text_dim));
            });
        });
        ui.add_space(12.0);
        ui.label(lang.tr("Edition author"));
        ui.label(egui::RichText::new("@CaioRods").strong());
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(6.0);
        ui.hyperlink_to(lang.tr("Licenses and attribution"), "https://github.com/CaioRods/Excel-AW/blob/aw-document-tabs/NOTICE");
    });
}
