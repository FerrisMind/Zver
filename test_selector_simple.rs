use zver::css::StyleEngine;

fn main() {
    let mut engine = StyleEngine::new();
    
    // Test simple pseudo-class
    let css1 = "li:first-child { color: red; }";
    match engine.parse_css(css1) {
        Ok(_) => println!("✓ :first-child parsed successfully"),
        Err(e) => println!("✗ :first-child failed: {:?}", e),
    }
    
    // Test functional pseudo-class
    let css2 = "li:nth-child(2) { color: green; }";
    match engine.parse_css(css2) {
        Ok(_) => println!("✓ :nth-child(2) parsed successfully"),
        Err(e) => println!("✗ :nth-child(2) failed: {:?}", e),
    }
}
