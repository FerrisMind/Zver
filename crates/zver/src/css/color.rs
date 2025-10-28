/// Представляет цвет в формате RGBA
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const YELLOW: Color = Color {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const GRAY: Color = Color {
        r: 128,
        g: 128,
        b: 128,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
}

/// Парсит CSS цвет в структуру Color
pub fn parse_css_color(color_str: &str) -> Option<Color> {
    let color_str = color_str.trim();

    // Hex формат: #RRGGBB или #RGB
    if let Some(hex) = color_str.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::rgb(r, g, b));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            return Some(Color::rgb(r, g, b));
        }
    }

    // rgb() формат: rgb(r, g, b)
    if let Some(rgb) = color_str.strip_prefix("rgb(")
        && let Some(rgb) = rgb.strip_suffix(')')
    {
        let parts: Vec<&str> = rgb.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            return Some(Color::rgb(r, g, b));
        }
    }

    // Именованные цвета
    match color_str.to_lowercase().as_str() {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "gray" | "grey" => Some(Color::GRAY),
        _ => None,
    }
}

/// Получает цвет по умолчанию для HTML тега
pub fn get_default_color_for_tag(tag_name: &Option<String>) -> Color {
    if let Some(tag) = tag_name {
        match tag.as_str() {
            "body" => Color::rgb(240, 240, 240),
            "h1" | "h2" | "h3" => Color::rgb(200, 220, 255),
            "div" => Color::rgb(220, 255, 220),
            "p" => Color::rgb(255, 255, 220),
            _ => Color::rgb(255, 240, 240),
        }
    } else {
        Color::rgb(255, 255, 200) // Текстовые узлы
    }
}
