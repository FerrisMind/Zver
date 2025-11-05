use std::collections::HashMap;

use zver::css::color::{Color, get_default_color_for_tag, parse_css_color};
use zver::layout::RenderInfo;
use zver::layout::render::get_debug_info;
use zver::layout::types::{ComputedStyle, FontStyle, FontWeight};

/// Конвертирует Color из движка в egui::Color32
pub fn color_to_egui(color: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Визуальный рендеринг layout результатов в egui с использованием painter
pub fn render_layout_results_in_painter(
    painter: &egui::Painter,
    offset: egui::Pos2,
    render_info: &[RenderInfo],
    resolved_styles: &HashMap<usize, ComputedStyle>,
    show_debug: bool,
) {
    use egui::{Color32, FontFamily, FontId, Rect, Stroke, Vec2};

    struct DebugOverlay {
        rect: Rect,
        label: String,
    }

    let mut debug_overlays = Vec::new();

    for info in render_info {
        let width = info.layout.width;
        let height = info.layout.height;

        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        let style = resolved_styles.get(&info.layout.node_id);
        let background_color = style
            .and_then(|s| s.background_color.as_deref())
            .and_then(parse_css_color)
            .map(color_to_egui)
            .or_else(|| {
                show_debug.then(|| color_to_egui(get_default_color_for_tag(&info.node.tag_name)))
            })
            .unwrap_or(Color32::TRANSPARENT);

        let rect = Rect::from_min_size(
            egui::pos2(offset.x + info.layout.x, offset.y + info.layout.y),
            Vec2::new(width.max(1.0), height.max(1.0)),
        );

        if background_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 0.0, background_color);
        }

        if show_debug {
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Color32::from_gray(60)),
                egui::StrokeKind::Outside,
            );

            if info.node.tag_name.is_some() && width > 50.0 && height > 15.0 {
                let label = format!(
                    "{} {}x{}",
                    get_debug_info(&info.node),
                    width as i32,
                    height as i32
                );
                debug_overlays.push(DebugOverlay { rect, label });
            }
        }
    }

    for info in render_info {
        if info.node.tag_name.is_some() {
            continue;
        }

        let width = info.layout.width;
        let height = info.layout.height;
        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        let text_content = info.node.text_content.clone().unwrap_or_default();
        let trimmed = text_content.trim();
        if trimmed.is_empty() {
            continue;
        }

        let style = resolved_styles.get(&info.layout.node_id);
        let text_color = style
            .and_then(|s| s.color.as_deref())
            .and_then(parse_css_color)
            .map(color_to_egui)
            .unwrap_or(Color32::BLACK);
        let font_size = style.map(|s| s.font_size).unwrap_or(16.0).clamp(8.0, 72.0);
        let font_family = FontFamily::Proportional;
        let font_id = FontId::new(font_size, font_family);
        let is_bold = style
            .map(|s| matches!(s.font_weight, FontWeight::Bold))
            .unwrap_or(false);

        use egui::text::{LayoutJob, TextFormat};

        let format = TextFormat {
            font_id,
            color: text_color,
            italics: style
                .map(|s| matches!(s.font_style, FontStyle::Italic))
                .unwrap_or(false),
            ..Default::default()
        };

        let mut job = LayoutJob::single_section(trimmed.to_owned(), format);
        // Используем content_width для правильного переноса текста
        job.wrap.max_width = info.layout.content_width.max(1.0);

        let galley = painter.fonts_mut(|fonts| fonts.layout_job(job));

        // Используем content_x/content_y для позиционирования текста внутри контентной области
        let text_pos = egui::pos2(
            offset.x + info.layout.content_x + 2.0,
            offset.y + info.layout.content_y + 2.0,
        );
        painter.galley(text_pos, galley.clone(), text_color);
        if is_bold {
            painter.galley(text_pos + Vec2::new(0.6, 0.0), galley, text_color);
        }
    }

    if show_debug {
        for overlay in debug_overlays {
            let width = (overlay.label.len() as f32 * 6.0 + 10.0).min(overlay.rect.width());
            let debug_bg = Rect::from_min_size(overlay.rect.min, Vec2::new(width, 14.0));
            painter.rect_filled(debug_bg, 0.0, Color32::from_black_alpha(96));

            let text_pos = overlay.rect.min + Vec2::new(2.0, 2.0);
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                overlay.label,
                egui::FontId::proportional(9.0),
                Color32::WHITE,
            );
        }
    }
}

/// Чистовой рендеринг без отладочной информации
pub fn render_clean_layout_from_results(
    painter: &egui::Painter,
    offset: egui::Pos2,
    render_info: &[RenderInfo],
    resolved_styles: &HashMap<usize, ComputedStyle>,
) {
    use egui::text::{LayoutJob, TextFormat};
    use egui::{Color32, FontFamily, FontId, Rect, Vec2};

    for info in render_info {
        let width = info.layout.width;
        let height = info.layout.height;

        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        if let Some(style) = resolved_styles.get(&info.layout.node_id)
            && let Some(color) = style
                .background_color
                .as_deref()
                .and_then(parse_css_color)
                .map(color_to_egui)
        {
            let rect = Rect::from_min_size(
                egui::pos2(offset.x + info.layout.x, offset.y + info.layout.y),
                Vec2::new(width.max(1.0), height.max(1.0)),
            );
            painter.rect_filled(rect, 0.0, color);
        }
    }

    for info in render_info {
        if info.node.tag_name.is_some() {
            continue;
        }

        let width = info.layout.width;
        let height = info.layout.height;
        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        let text_content = info.node.text_content.clone().unwrap_or_default();
        let trimmed = text_content.trim();
        if trimmed.is_empty() {
            continue;
        }

        let style = resolved_styles.get(&info.layout.node_id);
        let text_color = style
            .and_then(|s| s.color.as_deref())
            .and_then(parse_css_color)
            .map(color_to_egui)
            .unwrap_or(Color32::BLACK);
        let font_size = style.map(|s| s.font_size).unwrap_or(16.0).clamp(8.0, 72.0);
        let is_bold = style
            .map(|s| matches!(s.font_weight, FontWeight::Bold))
            .unwrap_or(false);

        let format = TextFormat {
            font_id: FontId::new(font_size, FontFamily::Proportional),
            color: text_color,
            italics: style
                .map(|s| matches!(s.font_style, FontStyle::Italic))
                .unwrap_or(false),
            ..Default::default()
        };

        let mut job = LayoutJob::single_section(trimmed.to_owned(), format);
        // Используем content_width для правильного переноса текста
        job.wrap.max_width = info.layout.content_width.max(1.0);

        let galley = painter.fonts_mut(|fonts| fonts.layout_job(job));
        // Используем content_x/content_y для позиционирования текста внутри контентной области
        let text_pos = egui::pos2(
            offset.x + info.layout.content_x + 2.0,
            offset.y + info.layout.content_y + 2.0,
        );

        painter.galley(text_pos, galley.clone(), text_color);
        if is_bold {
            painter.galley(text_pos + Vec2::new(0.6, 0.0), galley, text_color);
        }
    }
}
