use eframe::egui;
pub use egui_phosphor::variants::regular;

const PHOSPHOR_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Phosphor-Regular.ttf");

/// Registers the Phosphor regular font with egui so glyphs are available as icons.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(PHOSPHOR_FONT_BYTES).into(),
    );

    let proportional_family = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional_family.push("phosphor".into());

    ctx.set_fonts(fonts);
}
