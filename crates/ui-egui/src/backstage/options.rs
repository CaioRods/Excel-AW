use super::{Key, Language, Modifiers, SheetApp, Ui, dim, i18n, json, vec2};

use crate::theme::{self, Tokens};

pub(super) fn options(app: &mut SheetApp, ui: &mut Ui) {
    let lang = app.ui.language;
    settings_group(ui, lang, "Options|General", |ui| {
        general_options(app, ui, lang);
        ui.separator();
        let mut formula_bar = app.session.prefs.formula_bar;
        if ui.checkbox(&mut formula_bar, lang.tr("Show formula bar")).changed() {
            app.run_or_alert("view.formulaBar", json!({"on": formula_bar}));
        }
        let mut highlight_row = app.ui.highlight_active_row;
        if ui.checkbox(&mut highlight_row, lang.tr("Highlight the active cell's row")).changed() {
            app.run_or_alert("options.highlightActiveRow", json!({"on": highlight_row}));
        }
    });
    settings_group(ui, lang, "Editing and saving", |ui| {
        let mut autocomplete = app.ui.autocomplete;
        if ui.checkbox(&mut autocomplete, lang.tr("Enable AutoComplete for cell values")).changed() {
            app.run_or_alert("options.autoComplete", json!({"on": autocomplete}));
        }
        let mut autosave = app.ui.autosave;
        if ui.checkbox(&mut autosave, lang.tr("Save .xlsx workbooks automatically after every change")).changed() {
            app.toggle_autosave();
        }
    });
    let Some((calc, view)) = app.session.active().map(|d| (d.wb.calc.clone(), d.wb.view)) else { return };
    settings_group(ui, lang, "Display options for this workbook", |ui| {
        for (on, label, id) in [
            (view.show_horizontal_scroll, "Show horizontal scroll bar", "view.showHorizontalScrollBar"),
            (view.show_vertical_scroll, "Show vertical scroll bar", "view.showVerticalScrollBar"),
            (view.show_sheet_tabs, "Show sheet tabs", "view.showSheetTabs"),
        ] {
            let mut on = on;
            if ui.checkbox(&mut on, lang.tr(label)).changed() {
                app.run_or_alert(id, json!({"on": on}));
            }
        }
    });
    settings_group(ui, lang, "Formulas", |ui| {
        ui.label(lang.tr("Workbook calculation"));
        calc_options(app, ui, lang, &calc);
        ui.separator();
        dim(ui, lang.tr("Calculation settings are saved with the workbook."));
    });
}

fn settings_group(ui: &mut Ui, lang: Language, title: &str, contents: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    let width = ui.available_width().min(640.0);
    egui::Frame::new()
        .fill(t.menu_bg)
        .stroke(egui::Stroke::new(1.0, t.ribbon_border))
        .corner_radius(13)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width((width - 34.0).max(0.0));
            ui.spacing_mut().item_spacing.y = 7.0;
            ui.label(egui::RichText::new(lang.tr(title)).font(theme::ui_bold(15.0)).color(t.accent));
            ui.add_space(2.0);
            contents(ui);
        });
    ui.add_space(8.0);
}

/// The display theme and interface language controls of the General section.
fn general_options(app: &mut SheetApp, ui: &mut Ui, lang: Language) {
    egui::Grid::new("backstage_options_general").num_columns(2).spacing(vec2(18.0, 10.0)).show(ui, |ui| {
        let label = ui.label(lang.tr("Display theme"));
        let mode = app.ui.theme_mode();
        let current = match mode {
            "system" => "System",
            "dark" => "Dark",
            _ => "Light",
        };
        craft_ui::ComboBox::from_id_salt("backstage_theme")
            .width(160.0)
            .selected_text(lang.tr(current))
            .show_ui(ui, |ui| {
                for (value, label) in [("system", "System"), ("light", "Light"), ("dark", "Dark")] {
                    if ui.selectable_label(mode == value, lang.tr(label)).clicked() {
                        app.run_or_alert("view.theme", json!({"mode": value}));
                    }
                }
            })
            .response
            .labelled_by(label.id);
        ui.end_row();
        let label = ui.label(lang.tr("Interface language"));
        craft_ui::ComboBox::from_id_salt("backstage_language")
            .width(160.0)
            .selected_text(lang.name())
            .show_ui(ui, |ui| {
                for l in i18n::Language::for_picker() {
                    if ui.selectable_label(l == lang, l.name()).clicked() {
                        app.run_or_alert("app.language.set", json!({"language": l.code()}));
                    }
                }
            })
            .response
            .labelled_by(label.id);
        ui.end_row();
        let label = ui.label(lang.tr("Formula language"));
        let current = app.ui.formula_language;
        craft_ui::ComboBox::from_id_salt("formula_language")
            .width(160.0)
            .selected_text(current.map_or_else(|| lang.tr("Follow interface language"), Language::name))
            .show_ui(ui, |ui| {
                for choice in [None, Some(Language::PtBr), Some(Language::En), Some(Language::Es)] {
                    let name = choice.map_or_else(|| lang.tr("Follow interface language"), Language::name);
                    if ui.selectable_label(choice == current, name).clicked() {
                        app.ui.formula_language = choice;
                    }
                }
            })
            .response
            .labelled_by(label.id);
        ui.end_row();
    });
}

/// The calculation mode, and iterative calculation, of the active workbook.
fn calc_options(app: &mut SheetApp, ui: &mut Ui, lang: Language, calc: &gridcraft_engine::model::CalcSettings) {
    use gridcraft_engine::model::CalcMode;
    for (mode, id, label) in [
        (CalcMode::Automatic, "automatic", "Automatic"),
        (CalcMode::AutomaticExceptTables, "automaticExceptTables", "Automatic Except for Data Tables"),
        (CalcMode::Manual, "manual", "Manual"),
    ] {
        if ui.radio(calc.mode == mode, lang.tr(label)).clicked() && calc.mode != mode {
            app.run_or_alert("formulas.calculationOptions", json!({"mode": id}));
        }
    }
    ui.add_space(4.0);
    let mut iterative = calc.iterative;
    if ui.checkbox(&mut iterative, lang.tr("Enable iterative calculation")).changed() {
        app.run_or_alert("formulas.calculationOptions", json!({"iterative": iterative}));
    }
    // Maximum Iterations and Maximum Change apply only while iteration is on, as in Excel.
    iteration_options(app, ui, lang, calc, iterative);
    // TODO: add "Set precision as displayed" here once `formulas.calculationOptions` takes
    // `precisionAsDisplayed` (PR #1366); the model already stores it.
}

/// Maximum Iterations and Maximum Change, live only while iterative calculation is on.
fn iteration_options(app: &mut SheetApp, ui: &mut Ui, lang: Language, calc: &gridcraft_engine::model::CalcSettings, iterative: bool) {
    ui.add_enabled_ui(iterative, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            egui::Grid::new("backstage_options_iteration").num_columns(2).spacing(vec2(12.0, 6.0)).show(ui, |ui| {
                let label = ui.label(lang.tr("Maximum Iterations:"));
                let id = ui.make_persistent_id("backstage_max_iterations");
                // Text left in the field when iteration is switched off (it loses focus) is dropped.
                let committed = crate::widgets::committed_text(ui, id, &calc.max_iterations.to_string(), Some(80.0));
                if let Some(response) = ui.ctx().read_response(id) {
                    response.labelled_by(label.id);
                }
                if let Some(text) = committed.filter(|_| iterative) {
                    match parse_max_iterations(&text) {
                        Some(n) if n != calc.max_iterations => {
                            app.run_or_alert("formulas.calculationOptions", json!({"maxIterations": n}));
                        }
                        Some(_) => {}
                        None => refuse(app, ui, "Maximum Iterations must be a whole number from 1 to 32767."),
                    }
                }
                ui.end_row();
                let label = ui.label(lang.tr("Maximum Change:"));
                let id = ui.make_persistent_id("backstage_max_change");
                let committed = crate::widgets::committed_text(ui, id, &calc.max_change.to_string(), Some(80.0));
                if let Some(response) = ui.ctx().read_response(id) {
                    response.labelled_by(label.id);
                }
                if let Some(text) = committed.filter(|_| iterative) {
                    match parse_max_change(&text, lang.formula_locale()) {
                        Some(d) if d != calc.max_change => {
                            app.run_or_alert("formulas.calculationOptions", json!({"maxChange": d}));
                        }
                        Some(_) => {}
                        None => refuse(app, ui, "Maximum Change must be a number of 0 or more."),
                    }
                }
                ui.end_row();
            });
        });
    });
}

/// Refuses a typed setting with a message, leaving the workbook as it was. The Enter that
/// committed the field is consumed so it doesn't also dismiss the message box in this frame.
fn refuse(app: &mut SheetApp, ui: &mut Ui, text: &str) {
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
    app.message = Some(("Excel AW".into(), app.ui.language.tr(text).into()));
}

/// Maximum Iterations as typed: a whole number from 1 to 32767, or `None` (Excel refuses others).
fn parse_max_iterations(text: &str) -> Option<u32> {
    let n = text.trim().parse::<u32>().ok()?;
    (1..=gridcraft_engine::model::CalcSettings::MAX_ITERATIONS).contains(&n).then_some(n)
}

/// Maximum Change as typed: a finite number of 0 or more, or `None`. The interface's decimal
/// separator is accepted as well as a dot.
fn parse_max_change(text: &str, locale: gridcraft_engine::formula::FormulaLocale) -> Option<f64> {
    let d = text.trim().replace(locale.decimal_separator(), ".").parse::<f64>().ok()?;
    (d.is_finite() && d >= 0.0).then_some(d)
}

/// The platform the app was built for, e.g. "macOS (aarch64)"; the web build reports "Web".
pub(super) fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        "netbsd" => "NetBSD",
        _ if cfg!(target_arch = "wasm32") => return "Web".into(),
        other => other,
    };
    format!("{os} ({})", std::env::consts::ARCH)
}
