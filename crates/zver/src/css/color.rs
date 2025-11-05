use cssparser::{Parser, ParserInput, Token};

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

/// Парсит CSS цвет в структуру Color с использованием cssparser для компонентов
pub fn parse_css_color(color_str: &str) -> Option<Color> {
    let color_str = color_str.trim();

    // Hex формат: #RRGGBB или #RGB
    if let Some(hex) = color_str.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // rgb() и rgba() форматы с использованием cssparser для парсинга компонентов
    if let Some(rgb_content) = color_str
        .strip_prefix("rgb(")
        .or_else(|| color_str.strip_prefix("rgba("))
        && let Some(rgb_content) = rgb_content.strip_suffix(')')
    {
        return parse_rgb_color(rgb_content, color_str.starts_with("rgba("));
    }

    // hsl() и hsla() форматы
    if let Some(hsl_content) = color_str
        .strip_prefix("hsl(")
        .or_else(|| color_str.strip_prefix("hsla("))
        && let Some(hsl_content) = hsl_content.strip_suffix(')')
    {
        return parse_hsl_color(hsl_content, color_str.starts_with("hsla("));
    }

    // Именованные цвета
    parse_named_color(color_str)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color::new(r, g, b, a))
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
            Some(Color::new(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb_color(content: &str, has_alpha: bool) -> Option<Color> {
    let mut input = ParserInput::new(content);
    let mut parser = Parser::new(&mut input);

    // Парсим компоненты через запятую
    let components = parser
        .parse_comma_separated(|p| parse_color_component(p))
        .ok()?;

    if has_alpha && components.len() == 4 {
        let r = components[0] as u8;
        let g = components[1] as u8;
        let b = components[2] as u8;
        let a = (components[3] * 255.0).round() as u8;
        Some(Color::new(r, g, b, a))
    } else if !has_alpha && components.len() == 3 {
        let r = components[0] as u8;
        let g = components[1] as u8;
        let b = components[2] as u8;
        Some(Color::rgb(r, g, b))
    } else {
        None
    }
}

fn parse_hsl_color(content: &str, has_alpha: bool) -> Option<Color> {
    let mut input = ParserInput::new(content);
    let mut parser = Parser::new(&mut input);

    // Парсим компоненты HSL
    let components = parser
        .parse_comma_separated(|p| parse_color_component(p))
        .ok()?;

    if components.len() >= 3 {
        let h = components[0];
        let s = components[1] / 100.0; // Процент в дробь
        let l = components[2] / 100.0; // Процент в дробь

        // Конвертируем HSL в RGB
        let (r, g, b) = hsl_to_rgb(h, s, l);

        if has_alpha && components.len() == 4 {
            let a = (components[3] * 255.0).round() as u8;
            Some(Color::new(r, g, b, a))
        } else {
            Some(Color::rgb(r, g, b))
        }
    } else {
        None
    }
}

fn parse_color_component<'i>(
    parser: &mut Parser<'i, '_>,
) -> Result<f32, cssparser::ParseError<'i, ()>> {
    match parser.next()?.clone() {
        Token::Number { value, .. } => Ok(value),
        Token::Percentage { unit_value, .. } => Ok(unit_value * 255.0), // Процент в 0-255 для RGB
        Token::Dimension { value, unit, .. } => {
            if unit.as_ref().eq_ignore_ascii_case("deg") {
                Ok(value) // Градусы для HSL hue
            } else {
                Err(parser.new_custom_error(()))
            }
        }
        _ => Err(parser.new_custom_error(())),
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if (0.0..60.0).contains(&h) {
        (c, x, 0.0)
    } else if (60.0..120.0).contains(&h) {
        (x, c, 0.0)
    } else if (120.0..180.0).contains(&h) {
        (0.0, c, x)
    } else if (180.0..240.0).contains(&h) {
        (0.0, x, c)
    } else if (240.0..300.0).contains(&h) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round() as u8;
    let g = ((g_prime + m) * 255.0).round() as u8;
    let b = ((b_prime + m) * 255.0).round() as u8;

    (r, g, b)
}

fn parse_named_color(color_str: &str) -> Option<Color> {
    match color_str.to_lowercase().as_str() {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "gray" | "grey" => Some(Color::GRAY),
        "transparent" => Some(Color::TRANSPARENT),
        // Дополнительные веб-цвета
        "silver" => Some(Color::rgb(192, 192, 192)),
        "maroon" => Some(Color::rgb(128, 0, 0)),
        "purple" => Some(Color::rgb(128, 0, 128)),
        "fuchsia" => Some(Color::rgb(255, 0, 255)),
        "lime" => Some(Color::rgb(0, 255, 0)),
        "olive" => Some(Color::rgb(128, 128, 0)),
        "navy" => Some(Color::rgb(0, 0, 128)),
        "teal" => Some(Color::rgb(0, 128, 128)),
        "aqua" => Some(Color::rgb(0, 255, 255)),
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
