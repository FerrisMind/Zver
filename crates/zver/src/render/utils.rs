/// Вспомогательная функция для парсинга цвета
pub fn parse_color(color_str: &str) -> [f32; 4] {
    if let Some(hex) = color_str.strip_prefix('#')
        && hex.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        )
    {
        return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
    }

    // Цвета по умолчанию
    match color_str {
        "red" => [1.0, 0.0, 0.0, 1.0],
        "green" => [0.0, 1.0, 0.0, 1.0],
        "blue" => [0.0, 0.0, 1.0, 1.0],
        "white" => [1.0, 1.0, 1.0, 1.0],
        "black" => [0.0, 0.0, 0.0, 1.0],
        _ => [0.5, 0.5, 0.5, 1.0],
    }
}
