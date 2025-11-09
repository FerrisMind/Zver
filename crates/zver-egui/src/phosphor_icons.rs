use eframe::egui;

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
    proportional_family.insert(0, "phosphor".into());

    ctx.set_fonts(fonts);
}

/// Small selection of regularly-weighted Phosphor icon glyphs used by the UI.
pub mod regular {
    pub const GLOBE: &str = "\u{E288}";
    pub const ARROW_LEFT: &str = "\u{E058}";
    pub const ARROW_RIGHT: &str = "\u{E06C}";
    pub const ARROW_CLOCKWISE: &str = "\u{E036}";
    pub const BUG: &str = "\u{E5F4}";
    pub const MAGNIFYING_GLASS: &str = "\u{E30C}";
    pub const FILE: &str = "\u{E230}";
    pub const PLUS: &str = "\u{E3D4}";
    pub const X: &str = "\u{E4F6}";
    pub const LIST: &str = "\u{E2F0}";
}
