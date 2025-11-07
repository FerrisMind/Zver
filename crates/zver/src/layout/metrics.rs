/// Метрики шрифта для измерения текста
pub struct FontMetrics {
    pub char_width: f32,  // коэффициент ширины символа относительно font_size
    pub char_height: f32, // коэффициент высоты строки относительно font_size
}

impl FontMetrics {
    /// Создает стандартные метрики шрифта
    pub fn new() -> Self {
        Self {
            char_width: 0.6,  // эвристика: символ ≈ 0.6 от font_size
            char_height: 1.2, // line height ≈ 1.2 от font_size
        }
    }
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Контекст для измерения текстового узла
#[derive(Debug, Clone)]
pub struct TextMeasureContext {
    pub content: String,
    pub font_size: f32,
    pub font_weight: crate::layout::types::FontWeight,
    pub font_style: crate::layout::types::FontStyle,
}

/// Функция измерения текста для Taffy
pub fn text_measure_function(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<taffy::AvailableSpace>,
    node_context: Option<&TextMeasureContext>,
    font_metrics: &FontMetrics,
) -> taffy::Size<f32> {
    // Если размеры уже известны и положительные, возвращаем их
    if let taffy::Size {
        width: Some(width),
        height: Some(height),
    } = known_dimensions
        && width > 0.0
        && height > 0.0
    {
        return taffy::Size { width, height };
    }

    // Если нет текстового контекста, возвращаем нулевой размер
    let Some(text_ctx) = node_context else {
        return taffy::Size::ZERO;
    };

    // Применяем алгоритм измерения текста
    let words: Vec<&str> = text_ctx.content.split_whitespace().collect();
    if words.is_empty() {
        return taffy::Size::ZERO;
    }

    let char_width = text_ctx.font_size * font_metrics.char_width;
    let line_height = text_ctx.font_size * font_metrics.char_height;

    let min_line_length: usize = words.iter().map(|word| word.len()).max().unwrap_or(0);
    let max_line_length: usize =
        words.iter().map(|word| word.len()).sum::<usize>() + words.len().saturating_sub(1);

    let width = known_dimensions
        .width
        .unwrap_or_else(|| match available_space.width {
            taffy::AvailableSpace::MinContent => min_line_length as f32 * char_width,
            taffy::AvailableSpace::MaxContent => max_line_length as f32 * char_width,
            taffy::AvailableSpace::Definite(w) => w
                .min(max_line_length as f32 * char_width)
                .max(min_line_length as f32 * char_width),
        });

    let height = known_dimensions.height.unwrap_or_else(|| {
        let chars_per_line = (width / char_width).floor() as usize;
        if chars_per_line == 0 {
            return line_height;
        }

        let mut line_count = 1;
        let mut current_line_length = 0;

        for word in &words {
            if current_line_length == 0 {
                current_line_length = word.len();
            } else if current_line_length + word.len() + 1 > chars_per_line {
                line_count += 1;
                current_line_length = word.len();
            } else {
                current_line_length += word.len() + 1;
            }
        }

        line_count as f32 * line_height
    });

    taffy::Size { width, height }
}
