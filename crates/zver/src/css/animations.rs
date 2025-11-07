//! Парсинг и обработка CSS анимаций (@keyframes).
//!
//! Реализует поддержку W3C CSS Animations Level 1:
//! - @keyframes определения
//! - animation-* свойства
//! - Easing функции (timing functions)
//! - Интерполяция значений между keyframes
//!
//! Спецификация: https://www.w3.org/TR/css-animations-1/
//! Референс: https://developer.mozilla.org/en-US/docs/Web/CSS/@keyframes

#[allow(unused_imports)] // ParserInput используется в тестах
use cssparser::{ParseError, Parser, ParserInput, Token};
use std::fmt;

use super::properties::Property;

/// Один keyframe в анимации (точка во времени).
///
/// Например:
/// ```css
/// @keyframes slide {
///   0% { left: 0; }      // KeyframeStep { offset: 0.0, properties: [Property { name: "left", value: "0" }] }
///   100% { left: 100px; } // KeyframeStep { offset: 1.0, properties: [Property { name: "left", value: "100px" }] }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct KeyframeStep {
    /// Позиция во времени (0.0 = 0%, 1.0 = 100%)
    pub offset: f32,
    /// CSS-свойства, которые должны быть применены в этой точке
    pub properties: Vec<Property>,
}

impl KeyframeStep {
    /// Создает новый keyframe step.
    pub fn new(offset: f32) -> Self {
        Self {
            offset: offset.clamp(0.0, 1.0),
            properties: Vec::new(),
        }
    }

    /// Добавляет свойство к keyframe.
    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }

    /// Парсит offset из процентов или ключевых слов (from/to).
    ///
    /// Поддерживаемые форматы:
    /// - `0%`, `50%`, `100%`
    /// - `from` (эквивалент 0%)
    /// - `to` (эквивалент 100%)
    pub fn parse_offset<'i, 't>(input: &mut Parser<'i, 't>) -> Result<f32, ParseError<'i, ()>> {
        let token = input.next()?.clone();
        match token {
            Token::Percentage { unit_value, .. } => Ok(unit_value.clamp(0.0, 1.0)),
            Token::Ident(ref ident) => match ident.as_ref().to_ascii_lowercase().as_str() {
                "from" => Ok(0.0),
                "to" => Ok(1.0),
                _ => Err(input.new_custom_error(())),
            },
            _ => Err(input.new_custom_error(())),
        }
    }
}

/// Easing функция (timing function) для анимаций.
///
/// Определяет, как значение изменяется между keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingFunction {
    /// Линейная интерполяция (постоянная скорость)
    Linear,
    /// Плавный старт и конец (по умолчанию)
    Ease,
    /// Медленный старт
    EaseIn,
    /// Медленное завершение
    EaseOut,
    /// Медленный старт и конец (более выраженный, чем ease)
    EaseInOut,
    /// Кубическая кривая Безье: cubic-bezier(x1, y1, x2, y2)
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Пошаговая анимация: steps(n, start|end)
    Steps { count: u32, jump_start: bool },
}

impl Default for EasingFunction {
    fn default() -> Self {
        Self::Ease
    }
}

impl fmt::Display for EasingFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Ease => write!(f, "ease"),
            Self::EaseIn => write!(f, "ease-in"),
            Self::EaseOut => write!(f, "ease-out"),
            Self::EaseInOut => write!(f, "ease-in-out"),
            Self::CubicBezier { x1, y1, x2, y2 } => {
                write!(f, "cubic-bezier({}, {}, {}, {})", x1, y1, x2, y2)
            }
            Self::Steps { count, jump_start } => {
                write!(
                    f,
                    "steps({}, {})",
                    count,
                    if *jump_start { "start" } else { "end" }
                )
            }
        }
    }
}

impl EasingFunction {
    /// Вычисляет значение easing-функции для заданного прогресса (0.0 .. 1.0).
    ///
    /// Возвращает интерполированное значение (также 0.0 .. 1.0).
    pub fn apply(&self, progress: f32) -> f32 {
        let t = progress.clamp(0.0, 1.0);

        match self {
            Self::Linear => t,
            Self::Ease => {
                // cubic-bezier(0.25, 0.1, 0.25, 1.0)
                Self::cubic_bezier(t, 0.25, 0.1, 0.25, 1.0)
            }
            Self::EaseIn => {
                // cubic-bezier(0.42, 0, 1.0, 1.0)
                Self::cubic_bezier(t, 0.42, 0.0, 1.0, 1.0)
            }
            Self::EaseOut => {
                // cubic-bezier(0, 0, 0.58, 1.0)
                Self::cubic_bezier(t, 0.0, 0.0, 0.58, 1.0)
            }
            Self::EaseInOut => {
                // cubic-bezier(0.42, 0, 0.58, 1.0)
                Self::cubic_bezier(t, 0.42, 0.0, 0.58, 1.0)
            }
            Self::CubicBezier { x1, y1, x2, y2 } => Self::cubic_bezier(t, *x1, *y1, *x2, *y2),
            Self::Steps { count, jump_start } => {
                let steps = *count as f32;
                if *jump_start {
                    ((t * steps).ceil() / steps).min(1.0)
                } else {
                    ((t * steps).floor() / steps).max(0.0)
                }
            }
        }
    }

    /// Вычисляет кубическую кривую Безье (упрощенная реализация).
    ///
    /// Использует приближенный метод для вычисления y(t) по заданным контрольным точкам.
    ///
    /// # Note
    /// Это упрощенная версия, которая не учитывает x-координаты контрольных точек.
    /// Для production использования рассмотрите библиотеку `kurbo` или `lyon_geom`.
    ///
    /// # Arguments
    /// * `t` - прогресс времени (0.0..1.0)
    /// * `_x1`, `_x2` - x-координаты контрольных точек (не используются в упрощенной версии)
    /// * `y1`, `y2` - y-координаты контрольных точек
    fn cubic_bezier(t: f32, _x1: f32, y1: f32, _x2: f32, y2: f32) -> f32 {
        // Simplified cubic bezier calculation
        // TODO: Replace with proper cubic bezier solver that respects x1, x2
        // For accurate easing, we should solve x(t) = input_t for t, then compute y(t)
        // Current implementation only approximates the curve
        //
        // P(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
        // где P₀ = (0,0), P₁ = (x1,y1), P₂ = (x2,y2), P₃ = (1,1)

        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let _mt3 = mt2 * mt;

        // Вычисляем y-координату (без учёта x-координат контрольных точек)
        3.0 * mt2 * t * y1 + 3.0 * mt * t2 * y2 + t3
    }

    /// Парсит easing функцию из CSS-значения.
    pub fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
        // Пробуем распарсить как ident (linear, ease, etc.)
        let ident_result = input.try_parse(|i| {
            if let Token::Ident(ident) = i.next()? {
                Ok(ident.to_string())
            } else {
                Err(i.new_custom_error::<(), ()>(()))
            }
        });

        if let Ok(ident_str) = ident_result {
            return match ident_str.to_ascii_lowercase().as_str() {
                "linear" => Ok(Self::Linear),
                "ease" => Ok(Self::Ease),
                "ease-in" => Ok(Self::EaseIn),
                "ease-out" => Ok(Self::EaseOut),
                "ease-in-out" => Ok(Self::EaseInOut),
                _ => Err(input.new_custom_error(())),
            };
        }

        // Пробуем распарсить как функцию (cubic-bezier, steps)
        let function = input.expect_function()?.clone();
        input.parse_nested_block(|input| {
            match function.as_ref().to_ascii_lowercase().as_str() {
                "cubic-bezier" => {
                    let x1 = parse_number(input)?;
                    input.expect_comma()?;
                    let y1 = parse_number(input)?;
                    input.expect_comma()?;
                    let x2 = parse_number(input)?;
                    input.expect_comma()?;
                    let y2 = parse_number(input)?;
                    Ok(Self::CubicBezier { x1, y1, x2, y2 })
                }
                "steps" => {
                    let count = parse_integer(input)? as u32;
                    let jump_start = if input.try_parse(|i| i.expect_comma()).is_ok() {
                        let ident = input.expect_ident()?;
                        match ident.as_ref().to_ascii_lowercase().as_str() {
                            "start" => true,
                            "end" => false,
                            _ => return Err(input.new_custom_error(())),
                        }
                    } else {
                        false // По умолчанию "end"
                    };
                    Ok(Self::Steps { count, jump_start })
                }
                _ => Err(input.new_custom_error(())),
            }
        })
    }
}

/// Направление анимации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    /// Нормальное направление (от 0% к 100%)
    Normal,
    /// Обратное направление (от 100% к 0%)
    Reverse,
    /// Чередование: нормальное, потом обратное
    Alternate,
    /// Чередование с обратным стартом
    AlternateReverse,
}

impl Default for AnimationDirection {
    fn default() -> Self {
        Self::Normal
    }
}

/// Режим заполнения (fill mode) анимации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationFillMode {
    /// Без заполнения (анимация не влияет на стили вне времени выполнения)
    None,
    /// Применять стили первого keyframe до старта
    Forwards,
    /// Применять стили последнего keyframe после завершения
    Backwards,
    /// Применять оба
    Both,
}

impl Default for AnimationFillMode {
    fn default() -> Self {
        Self::None
    }
}

/// Определение @keyframes анимации.
///
/// Содержит имя анимации и список keyframe steps.
#[derive(Debug, Clone)]
pub struct KeyframesDefinition {
    /// Имя анимации (используется в animation-name)
    pub name: String,
    /// Список ключевых кадров, отсортированных по offset
    pub steps: Vec<KeyframeStep>,
}

impl KeyframesDefinition {
    /// Создает новое определение @keyframes.
    pub fn new(name: String) -> Self {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    /// Добавляет keyframe step и сортирует список по offset.
    ///
    /// # Panics
    /// Не паникует - NaN значения обрабатываются как равные (Equal ordering).
    pub fn add_step(&mut self, step: KeyframeStep) {
        self.steps.push(step);
        self.steps.sort_by(|a, b| {
            a.offset
                .partial_cmp(&b.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Находит два keyframe между которыми находится заданный прогресс.
    ///
    /// Возвращает (предыдущий keyframe, следующий keyframe, локальный прогресс между ними).
    pub fn find_interpolation_range(
        &self,
        progress: f32,
    ) -> Option<(&KeyframeStep, &KeyframeStep, f32)> {
        if self.steps.len() < 2 {
            return None;
        }

        for i in 0..self.steps.len() - 1 {
            let current = &self.steps[i];
            let next = &self.steps[i + 1];

            if progress >= current.offset && progress <= next.offset {
                let range = next.offset - current.offset;
                let local_progress = if range > 0.0 {
                    (progress - current.offset) / range
                } else {
                    0.0
                };
                return Some((current, next, local_progress));
            }
        }

        None
    }

    /// Парсит @keyframes из cssparser::Parser.
    ///
    /// Ожидает содержимое блока @keyframes (без самого @keyframes и имени).
    pub fn parse_keyframes_block<'i, 't>(
        name: String,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self, ParseError<'i, ()>> {
        let mut definition = Self::new(name);

        // Парсим каждый keyframe block
        while !input.is_exhausted() {
            input.skip_whitespace();

            // Парсим селектор keyframe (может быть несколько через запятую)
            // Например: "from, 50%, to" или просто "50%"
            let mut offsets = Vec::new();
            loop {
                let offset = KeyframeStep::parse_offset(input)?;
                offsets.push(offset);

                input.skip_whitespace();

                // Проверяем, есть ли запятая для следующего offset
                if input.try_parse(|i| i.expect_comma()).is_err() {
                    break;
                }

                input.skip_whitespace();
            }

            input.skip_whitespace();

            // Парсим блок с декларациями
            // Ожидаем CurlyBracketBlock токен, затем парсим его содержимое
            let properties = match input.next() {
                Ok(Token::CurlyBracketBlock) => {
                    input.parse_nested_block(|nested| parse_keyframe_declarations(nested))?
                }
                Ok(other) => {
                    eprintln!("Expected CurlyBracketBlock, got {:?}", other);
                    return Err(input.new_custom_error(()));
                }
                Err(e) => {
                    eprintln!("Failed to get next token for keyframe block: {:?}", e);
                    return Err(input.new_error(e.kind));
                }
            };

            // Создаем keyframe step для каждого offset
            for offset in offsets {
                let mut step = KeyframeStep::new(offset);
                step.properties = properties.clone();
                definition.add_step(step);
            }

            input.skip_whitespace();
        }

        Ok(definition)
    }
}

/// Конфигурация анимации (соответствует animation-* свойствам).
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Имя анимации (ссылка на @keyframes)
    pub name: String,
    /// Длительность в секундах
    pub duration: f32,
    /// Функция easing
    pub timing_function: EasingFunction,
    /// Задержка перед стартом в секундах
    pub delay: f32,
    /// Количество повторений (f32::INFINITY для infinite)
    pub iteration_count: f32,
    /// Направление анимации
    pub direction: AnimationDirection,
    /// Режим заполнения
    pub fill_mode: AnimationFillMode,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            duration: 0.0,
            timing_function: EasingFunction::default(),
            delay: 0.0,
            iteration_count: 1.0,
            direction: AnimationDirection::default(),
            fill_mode: AnimationFillMode::default(),
        }
    }
}

impl AnimationConfig {
    /// Создает новую конфигурацию анимации.
    pub fn new(name: String, duration: f32) -> Self {
        Self {
            name,
            duration,
            ..Default::default()
        }
    }
}

/// Парсит декларации внутри keyframe блока.
fn parse_keyframe_declarations<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Vec<Property>, ParseError<'i, ()>> {
    let mut properties = Vec::new();

    while !input.is_exhausted() {
        input.skip_whitespace();

        // Парсим имя свойства
        let name = match input.next() {
            Ok(Token::Ident(ident)) => ident.to_string(),
            Ok(_) => continue,
            Err(_) => break,
        };

        input.skip_whitespace();

        // Ожидаем двоеточие
        if input.expect_colon().is_err() {
            continue;
        }

        input.skip_whitespace();

        // Парсим значение до точки с запятой или конца блока
        // Используем правильную сериализацию вместо Debug форматирования
        let value = match super::serializer::serialize_value_tokens(input, true) {
            Ok(v) => v,
            Err(e) => {
                // Логируем ошибку парсинга, но не паникуем
                #[cfg(debug_assertions)]
                eprintln!(
                    "Warning: Failed to serialize CSS value for '{}': {:?}",
                    name, e
                );
                String::new()
            }
        };

        if !value.is_empty() {
            properties.push(Property {
                name,
                value,
                important: false,
            });
        }

        input.skip_whitespace();
    }

    Ok(properties)
}

/// Парсит число с плавающей точкой.
fn parse_number<'i, 't>(input: &mut Parser<'i, 't>) -> Result<f32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match token {
        Token::Number { value, .. } => Ok(value),
        _ => Err(input.new_custom_error(())),
    }
}

/// Парсит целое число.
fn parse_integer<'i, 't>(input: &mut Parser<'i, 't>) -> Result<i32, ParseError<'i, ()>> {
    let token = input.next()?.clone();
    match token {
        Token::Number {
            int_value: Some(i), ..
        } => Ok(i),
        _ => Err(input.new_custom_error(())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        let easing = EasingFunction::Linear;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(0.5), 0.5);
        assert_eq!(easing.apply(1.0), 1.0);
    }

    #[test]
    fn test_easing_steps() {
        let easing = EasingFunction::Steps {
            count: 4,
            jump_start: false,
        };
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(0.24), 0.0);
        assert_eq!(easing.apply(0.25), 0.25);
        assert_eq!(easing.apply(0.5), 0.5);
        assert_eq!(easing.apply(0.75), 0.75);
    }

    #[test]
    fn test_keyframe_offset_parsing() {
        let css = "50%";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);

        let offset = KeyframeStep::parse_offset(&mut parser).unwrap();
        assert_eq!(offset, 0.5);

        let css = "from";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let offset = KeyframeStep::parse_offset(&mut parser).unwrap();
        assert_eq!(offset, 0.0);

        let css = "to";
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        let offset = KeyframeStep::parse_offset(&mut parser).unwrap();
        assert_eq!(offset, 1.0);
    }
}
