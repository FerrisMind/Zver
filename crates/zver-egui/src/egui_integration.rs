use zver::css::color::{Color, get_default_color_for_tag, parse_css_color};
use zver::dom::Document;
use zver::layout::LayoutNode;

/// Получает отладочную информацию о узле
fn get_debug_info(node: &LayoutNode, dom: &Document) -> String {
    if let Some(dom_node) = dom.nodes.get(&node.dom_node) {
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

/// Визуальный рендеринг layout tree в egui с использованием painter
pub fn render_layout_tree_in_painter(
    painter: &egui::Painter,
    offset: egui::Pos2,
    node: &LayoutNode,
    dom: &Document,
    _depth: usize,
) {
    use egui::{Color32, Rect, Stroke, Vec2};

    let x = node.dimensions.x;
    let y = node.dimensions.y;
    let width = node.dimensions.width;
    let height = node.dimensions.height;

    // Пропускаем узлы с нулевыми размерами
    if width <= 0.0 || height <= 0.0 {
        for child in &node.children {
            render_layout_tree_in_painter(painter, offset, child, dom, 0);
        }
        return;
    }

    // Определяем цвет на основе CSS стилей или типа узла
    let (bg_color, text_info, text_content) = if let Some(dom_node) = dom.nodes.get(&node.dom_node)
    {
        let bg = if let Some(bg_css) = &node.style.background_color {
            parse_css_color(bg_css)
                .map(color_to_egui)
                .unwrap_or_else(|| color_to_egui(get_default_color_for_tag(&dom_node.tag_name)))
        } else {
            color_to_egui(get_default_color_for_tag(&dom_node.tag_name))
        };

        let info = get_debug_info(node, dom);
        let text = dom_node.text_content.clone();

        (bg, info, text)
    } else {
        (Color32::WHITE, "unknown".to_string(), None)
    };

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
        let is_text_node = if let Some(dom_node) = dom.nodes.get(&node.dom_node) {
            dom_node.tag_name.is_none()
        } else {
            false
        };

        if is_text_node && !trimmed.is_empty() && width > 10.0 && height > 10.0 {
            let text_color = if let Some(color_css) = &node.style.color {
                parse_css_color(color_css)
                    .map(color_to_egui)
                    .unwrap_or(Color32::BLACK)
            } else {
                Color32::BLACK
            };

            let font_size = node.style.font_size.clamp(8.0, 32.0);
            let text_pos = rect.min + Vec2::new(4.0, 4.0);

            let font_id = match node.style.font_weight {
                zver::layout::FontWeight::Bold => egui::FontId::monospace(font_size),
                _ => egui::FontId::proportional(font_size),
            };

            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                trimmed,
                font_id,
                text_color,
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

    // Рекурсивно рендерим дочерние узлы
    for child in &node.children {
        render_layout_tree_in_painter(painter, offset, child, dom, 0);
    }
}

/// Чистовой рендеринг без отладочной информации
pub fn render_clean_layout(
    painter: &egui::Painter,
    offset: egui::Pos2,
    node: &LayoutNode,
    dom: &Document,
) {
    use egui::{Color32, Rect, Vec2};

    let x = node.dimensions.x;
    let y = node.dimensions.y;
    let width = node.dimensions.width;
    let height = node.dimensions.height;

    // Пропускаем узлы с нулевыми размерами
    if width <= 0.0 || height <= 0.0 {
        for child in &node.children {
            render_clean_layout(painter, offset, child, dom);
        }
        return;
    }

    // Получаем DOM узел и его информацию
    let (bg_color, text_content, tag_name) = if let Some(dom_node) = dom.nodes.get(&node.dom_node) {
        let bg = if let Some(bg_css) = &node.style.background_color {
            parse_css_color(bg_css)
                .map(color_to_egui)
                .unwrap_or(Color32::TRANSPARENT)
        } else {
            Color32::TRANSPARENT
        };

        let text = dom_node.text_content.clone().unwrap_or_default();
        (bg, text, dom_node.tag_name.clone())
    } else {
        (Color32::TRANSPARENT, String::new(), None)
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
        let text_color = if let Some(color_css) = &node.style.color {
            parse_css_color(color_css)
                .map(color_to_egui)
                .unwrap_or(Color32::BLACK)
        } else {
            Color32::BLACK
        };

        // Определяем размер шрифта в зависимости от тега
        let mut font_size = node.style.font_size;
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

        let font_id = match node.style.font_weight {
            zver::layout::FontWeight::Bold => egui::FontId::monospace(font_size),
            _ => egui::FontId::proportional(font_size),
        };

        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            trimmed,
            font_id,
            text_color,
        );
    }

    // Рекурсивно рендерим дочерние узлы
    for child in &node.children {
        render_clean_layout(painter, offset, child, dom);
    }
}
