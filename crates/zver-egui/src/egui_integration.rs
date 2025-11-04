use zver::css::color::{Color, get_default_color_for_tag, parse_css_color};
use zver::dom::Document;
use zver::layout::RenderInfo;

/// Получает отладочную информацию о узле по его ID
fn get_debug_info(node_id: usize, dom: &Document) -> String {
    if let Some(dom_node) = dom.nodes.get(&node_id) {
        if let Some(tag) = &dom_node.tag_name {
            format!("<{}>", tag)
        } else if let Some(text) = &dom_node.text_content {
            let trimmed = text.trim();
            let char_count = trimmed.chars().count();
            if char_count > 30 {
                let truncated: String = trimmed.chars().take(30).collect();
                format!("\"{}...\"", truncated)
            } else {
                format!("\"{}\"", trimmed)
            }
        } else {
            "node".to_string()
        }
    } else {
        "unknown".to_string()
    }
}

/// Конвертирует Color в egui::Color32
pub fn color_to_egui(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Визуальный рендеринг layout результатов в egui с использованием painter
pub fn render_layout_results_in_painter(
    painter: &egui::Painter,
    offset: egui::Pos2,
    render_info: &[RenderInfo],
    dom: &Document,
) {
    use egui::{Color32, Rect, Stroke, Vec2};

    for info in render_info {
        let x = info.layout.x;
        let y = info.layout.y;
        let width = info.layout.width;
        let height = info.layout.height;

        // Пропускаем узлы с нулевыми размерами
        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        // Определяем цвет на основе CSS стилей или типа узла
        let bg_color = if let Some(parsed_bg_color) = info
            .node
            .attributes
            .get("style")
            .and_then(|style| {
                // Простой парсинг background-color из inline стиля
                style
                    .split(';')
                    .find(|prop| prop.trim().starts_with("background-color:"))
                    .and_then(|prop| prop.split(':').nth(1))
                    .map(|val| val.trim())
            })
            .and_then(parse_css_color)
        {
            color_to_egui(parsed_bg_color)
        } else {
            color_to_egui(get_default_color_for_tag(&info.node.tag_name))
        };

        let text_info = get_debug_info(info.layout.node_id, dom);
        let text_content = info.node.text_content.clone();

        // Рисуем прямоугольник для этого узла
        let rect = Rect::from_min_size(
            egui::pos2(offset.x + x, offset.y + y),
            Vec2::new(width.max(1.0), height.max(1.0)),
        );

        painter.rect_filled(rect, 0.0, bg_color);
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::DARK_GRAY),
            egui::StrokeKind::Outside,
        );

        // Рендерим текст ТОЛЬКО если он есть в этом конкретном узле
        if let Some(text) = text_content {
            let trimmed = text.trim();
            let is_text_node = info.node.tag_name.is_none();

            if is_text_node && !trimmed.is_empty() && width > 10.0 && height > 10.0 {
                let text_color_parsed = if let Some(parsed_color) = info
                    .node
                    .attributes
                    .get("style")
                    .and_then(|style| {
                        // Простой парсинг color из inline стиля
                        style
                            .split(';')
                            .find(|prop| prop.trim().starts_with("color:"))
                            .and_then(|prop| prop.split(':').nth(1))
                            .map(|val| val.trim())
                    })
                    .and_then(parse_css_color)
                {
                    color_to_egui(parsed_color)
                } else {
                    Color32::BLACK
                };

                // Получаем font-size из стилей (простая эвристика)
                let font_size = info
                    .node
                    .attributes
                    .get("style")
                    .and_then(|style| {
                        style
                            .split(';')
                            .find(|prop| prop.trim().starts_with("font-size:"))
                            .and_then(|prop| prop.split(':').nth(1))
                            .and_then(|size| size.trim().strip_suffix("px"))
                            .and_then(|size| size.parse::<f32>().ok())
                    })
                    .unwrap_or(16.0)
                    .clamp(8.0, 32.0);

                let text_pos = rect.min + Vec2::new(4.0, 4.0);

                // Определяем тип шрифта на основе font-weight
                let font_weight = info.node.attributes.get("style").and_then(|style| {
                    style
                        .split(';')
                        .find(|prop| prop.trim().starts_with("font-weight:"))
                        .and_then(|prop| prop.split(':').nth(1))
                        .map(|weight| weight.trim())
                });

                let font_id = if font_weight.is_some_and(|w| w == "bold" || w == "700") {
                    egui::FontId::monospace(font_size)
                } else {
                    egui::FontId::proportional(font_size)
                };

                painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    trimmed,
                    font_id,
                    text_color_parsed,
                );
            }
        } else {
            // Добавляем отладочную информацию только для контейнеров без текста
            if width > 50.0 && height > 15.0 {
                let debug_bg = Rect::from_min_size(
                    rect.min,
                    Vec2::new((text_info.len() as f32 * 6.0 + 10.0).min(width), 14.0),
                );
                painter.rect_filled(debug_bg, 0.0, Color32::from_black_alpha(128));

                let text_pos = rect.min + Vec2::new(2.0, 2.0);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    format!("{} {}x{}", text_info, width as i32, height as i32),
                    egui::FontId::proportional(9.0),
                    Color32::WHITE,
                );
            }
        }
    }
}

/// Чистовой рендеринг без отладочной информации
pub fn render_clean_layout_from_results(
    painter: &egui::Painter,
    offset: egui::Pos2,
    render_info: &[RenderInfo],
    _dom: &Document,
) {
    use egui::{Color32, Rect, Vec2};

    for info in render_info {
        let x = info.layout.x;
        let y = info.layout.y;
        let width = info.layout.width;
        let height = info.layout.height;

        // Пропускаем узлы с нулевыми размерами
        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        // Получаем DOM узел и его информацию
        let (bg_color, text_content, tag_name) = {
            let bg_color_parsed = if let Some(parsed_bg_color) = info
                .node
                .attributes
                .get("style")
                .and_then(|style| {
                    style
                        .split(';')
                        .find(|prop| prop.trim().starts_with("background-color:"))
                        .and_then(|prop| prop.split(':').nth(1))
                        .map(|val| val.trim())
                })
                .and_then(parse_css_color)
            {
                color_to_egui(parsed_bg_color)
            } else {
                Color32::TRANSPARENT
            };

            let text = info.node.text_content.clone().unwrap_or_default();
            (bg_color_parsed, text, info.node.tag_name.clone())
        };

        // Рисуем прямоугольник только если есть цвет фона
        if bg_color != Color32::TRANSPARENT {
            let rect = Rect::from_min_size(
                egui::pos2(offset.x + x, offset.y + y),
                Vec2::new(width.max(1.0), height.max(1.0)),
            );
            painter.rect_filled(rect, 0.0, bg_color);
        }

        // Рендерим текстовый контент
        let trimmed = text_content.trim();
        let is_text_node = tag_name.is_none();

        if is_text_node && !trimmed.is_empty() && width > 10.0 && height > 10.0 {
            let text_color_parsed = if let Some(parsed_text_color) = info
                .node
                .attributes
                .get("style")
                .and_then(|style| {
                    style
                        .split(';')
                        .find(|prop| prop.trim().starts_with("color:"))
                        .and_then(|prop| prop.split(':').nth(1))
                        .map(|val| val.trim())
                })
                .and_then(parse_css_color)
            {
                color_to_egui(parsed_text_color)
            } else {
                Color32::BLACK
            };

            // Определяем размер шрифта в зависимости от тега
            let mut font_size = info
                .node
                .attributes
                .get("style")
                .and_then(|style| {
                    style
                        .split(';')
                        .find(|prop| prop.trim().starts_with("font-size:"))
                        .and_then(|prop| prop.split(':').nth(1))
                        .and_then(|size| size.trim().strip_suffix("px"))
                        .and_then(|size| size.parse::<f32>().ok())
                })
                .unwrap_or(16.0);

            if let Some(tag) = &tag_name {
                font_size = match tag.as_str() {
                    "h1" => 32.0,
                    "h2" => 28.0,
                    "h3" => 24.0,
                    "h4" => 20.0,
                    "h5" => 18.0,
                    "h6" => 16.0,
                    _ => font_size,
                };
            }

            font_size = font_size.clamp(8.0, 48.0);

            let rect = Rect::from_min_size(
                egui::pos2(offset.x + x, offset.y + y),
                Vec2::new(width, height),
            );
            let text_pos = rect.min + Vec2::new(4.0, 4.0);

            // Определяем тип шрифта на основе font-weight
            let font_weight = info.node.attributes.get("style").and_then(|style| {
                style
                    .split(';')
                    .find(|prop| prop.trim().starts_with("font-weight:"))
                    .and_then(|prop| prop.split(':').nth(1))
                    .map(|weight| weight.trim())
            });

            let font_id = if font_weight.is_some_and(|w| w == "bold" || w == "700") {
                egui::FontId::monospace(font_size)
            } else {
                egui::FontId::proportional(font_size)
            };

            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                trimmed,
                font_id,
                text_color_parsed,
            );
        }
    }
}
