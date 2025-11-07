use zver::css::parser::CssParser;

fn main() {
    env_logger::init();
    
    let css = r#"
        @font-face {
            font-family: 'TestFont';
            src: url('../assets/fonts/Roboto-Regular.ttf') format('truetype');
            font-weight: normal;
            font-style: normal;
        }
    "#;
    
    let parser = CssParser::new();
    match parser.parse_stylesheet(css) {
        Ok(stylesheet) => {
            println!("✓ Успешно распарсили @font-face!");
            println!("Правил: {}", stylesheet.rules.len());
        }
        Err(e) => {
            eprintln!("✗ Ошибка парсинга: {}", e);
        }
    }
}
