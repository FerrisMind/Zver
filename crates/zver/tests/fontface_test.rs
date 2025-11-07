#[cfg(test)]
mod fontface_test {
    use zver::css::parser::{StylesheetParser, CssParseOptions};

    #[test]
    fn test_font_face_with_format() {
        let css = r#"
            @font-face {
                font-family: 'TestFont';
                src: url('../assets/fonts/Roboto-Regular.ttf') format('truetype');
                font-weight: normal;
                font-style: normal;
            }
        "#;
        
        let mut parser = StylesheetParser::new(CssParseOptions::default());
        let result = parser.parse_stylesheet(css);
        println!("Результат парсинга: {:?}", result);
        
        if let Err(ref e) = result {
            eprintln!("Ошибка: {}", e);
        }
        
        assert!(result.is_ok(), "Парсинг должен быть успешным");
        
        let stylesheet = result.unwrap();
        println!("@font-face правил: {}", stylesheet.font_faces.len());
        assert_eq!(stylesheet.font_faces.len(), 1, "Должно быть 1 @font-face правило");
    }

    #[test]
    fn test_font_face_without_format() {
        let css = r#"
            @font-face {
                font-family: 'TestFont';
                src: url('../assets/fonts/Roboto-Regular.ttf');
                font-weight: normal;
            }
        "#;
        
        let mut parser = StylesheetParser::new(CssParseOptions::default());
        let result = parser.parse_stylesheet(css);
        
        if let Err(ref e) = result {
            eprintln!("Ошибка: {}", e);
        }
        
        assert!(result.is_ok(), "Парсинг без format() должен работать");
    }

    #[test]
    fn test_font_face_unicode_range() {
        // Точная копия из phase2_font_face.html
        let css = r#"
            @font-face {
                font-family: 'TestFontUnicode';
                src: url('../assets/fonts/Roboto-Regular.ttf') format('truetype');
                font-weight: normal;
                font-style: normal;
                unicode-range: U+0020-007F; /* Latin */
            }
        "#;
        
        let mut parser = StylesheetParser::new(CssParseOptions::default());
        let result = parser.parse_stylesheet(css);
        
        if let Err(ref e) = result {
            eprintln!("Ошибка: {}", e);
        }
        
        assert!(result.is_ok(), "Парсинг с unicode-range должен работать");
        
        let stylesheet = result.unwrap();
        assert_eq!(stylesheet.font_faces.len(), 1);
        println!("@font-face: {:?}", stylesheet.font_faces[0]);
    }
}
