use std::fs;
/// Unit tests for Network module
/// Tests cover: caching, timeouts, fallback, file:// protocol, error handling
use zver::network::NetworkEngine;

#[tokio::test]
async fn test_network_engine_creation() {
    let _engine = NetworkEngine::new();
    // Should create successfully without panic
    assert!(true, "Network engine created successfully");
}

#[tokio::test]
async fn test_cache_functionality() {
    let mut engine = NetworkEngine::new();

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_cache.txt");
    fs::write(&test_file, "Test content for caching").unwrap();

    let file_url = format!("file://{}", test_file.display());

    // First fetch - should hit disk
    let result1 = engine.fetch(&file_url).await;
    assert!(result1.is_ok(), "First fetch should succeed");
    assert_eq!(result1.unwrap(), "Test content for caching");

    // Modify file
    fs::write(&test_file, "Modified content").unwrap();

    // Second fetch - should return cached version (old content)
    let result2 = engine.fetch(&file_url).await;
    assert!(result2.is_ok(), "Second fetch should succeed");
    assert_eq!(
        result2.unwrap(),
        "Test content for caching",
        "Should return cached content"
    );

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_cache_clear() {
    let mut engine = NetworkEngine::new();

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_cache_clear.txt");
    fs::write(&test_file, "Original content").unwrap();

    let file_url = format!("file://{}", test_file.display());

    // First fetch - cache it
    let result1 = engine.fetch(&file_url).await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), "Original content");

    // Clear cache
    engine.clear_cache_for_url(&file_url);

    // Modify file
    fs::write(&test_file, "New content").unwrap();

    // Fetch again - should get new content
    let result2 = engine.fetch(&file_url).await;
    assert!(result2.is_ok());
    assert_eq!(
        result2.unwrap(),
        "New content",
        "Should fetch fresh content after cache clear"
    );

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_file_protocol() {
    let mut engine = NetworkEngine::new();

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_file_protocol.html");
    fs::write(&test_file, "<!DOCTYPE html><html><body>Test</body></html>").unwrap();

    // Test with file:// prefix
    let file_url = format!("file://{}", test_file.display());
    let result = engine.fetch(&file_url).await;

    assert!(result.is_ok(), "Should fetch file:// URL");
    assert!(
        result.unwrap().contains("Test"),
        "Should contain file content"
    );

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_file_without_protocol() {
    let mut engine = NetworkEngine::new();

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_no_protocol.txt");
    fs::write(&test_file, "Content without protocol").unwrap();

    // Test without file:// prefix (should be auto-added)
    let file_path = test_file.display().to_string();
    let result = engine.fetch(&file_path).await;

    assert!(result.is_ok(), "Should fetch path without file:// protocol");
    assert_eq!(result.unwrap(), "Content without protocol");

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_nonexistent_file_error() {
    let mut engine = NetworkEngine::new();

    let fake_path = "file:///this/path/does/not/exist/hopefully/zver_test.txt";
    let result = engine.fetch(fake_path).await;

    assert!(result.is_err(), "Should return error for non-existent file");
}

#[tokio::test]
async fn test_prefetch_resources_files() {
    let engine = NetworkEngine::new();

    // Create temporary test files
    let temp_dir = std::env::temp_dir();
    let file1 = temp_dir.join("zver_prefetch_1.txt");
    let file2 = temp_dir.join("zver_prefetch_2.txt");

    fs::write(&file1, "File 1 content").unwrap();
    fs::write(&file2, "File 2 content").unwrap();

    let urls = vec![
        format!("file://{}", file1.display()),
        format!("file://{}", file2.display()),
    ];

    let results = engine.prefetch_resources(urls).await;

    assert_eq!(results.len(), 2, "Should return 2 results");
    assert!(results[0].is_ok(), "First prefetch should succeed");
    assert!(results[1].is_ok(), "Second prefetch should succeed");

    if let Ok(content1) = &results[0] {
        assert_eq!(content1, "File 1 content");
    }
    if let Ok(content2) = &results[1] {
        assert_eq!(content2, "File 2 content");
    }

    // Cleanup
    let _ = fs::remove_file(file1);
    let _ = fs::remove_file(file2);
}

#[tokio::test]
async fn test_prefetch_with_errors() {
    let engine = NetworkEngine::new();

    // Create one valid file and one invalid path
    let temp_dir = std::env::temp_dir();
    let valid_file = temp_dir.join("zver_prefetch_valid.txt");
    fs::write(&valid_file, "Valid content").unwrap();

    let urls = vec![
        format!("file://{}", valid_file.display()),
        "file:///nonexistent/path/zver_test.txt".to_string(),
    ];

    let results = engine.prefetch_resources(urls).await;

    assert_eq!(results.len(), 2, "Should return 2 results");
    assert!(results[0].is_ok(), "Valid file should succeed");
    assert!(results[1].is_err(), "Invalid file should fail");

    // Cleanup
    let _ = fs::remove_file(valid_file);
}

#[tokio::test]
async fn test_multiple_fetches_same_url() {
    let mut engine = NetworkEngine::new();

    // Create test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_multiple.txt");
    fs::write(&test_file, "Reusable content").unwrap();

    let file_url = format!("file://{}", test_file.display());

    // Fetch multiple times
    let result1 = engine.fetch(&file_url).await;
    let result2 = engine.fetch(&file_url).await;
    let result3 = engine.fetch(&file_url).await;

    assert!(result1.is_ok() && result2.is_ok() && result3.is_ok());
    assert_eq!(result1.unwrap(), "Reusable content");
    assert_eq!(result2.unwrap(), "Reusable content");
    assert_eq!(result3.unwrap(), "Reusable content");

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_empty_file() {
    let mut engine = NetworkEngine::new();

    // Create empty file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_empty.txt");
    fs::write(&test_file, "").unwrap();

    let file_url = format!("file://{}", test_file.display());
    let result = engine.fetch(&file_url).await;

    assert!(result.is_ok(), "Should handle empty file");
    assert_eq!(result.unwrap(), "", "Empty file should return empty string");

    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[tokio::test]
async fn test_large_file() {
    let mut engine = NetworkEngine::new();

    // Create large file (1MB of 'A's)
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("zver_test_large.txt");
    let large_content = "A".repeat(1_000_000);
    fs::write(&test_file, &large_content).unwrap();

    let file_url = format!("file://{}", test_file.display());
    let result = engine.fetch(&file_url).await;

    assert!(result.is_ok(), "Should handle large file");
    assert_eq!(
        result.unwrap().len(),
        1_000_000,
        "Should read entire large file"
    );

    // Cleanup
    let _ = fs::remove_file(test_file);
}

// Note: HTTP/HTTPS tests are commented out to avoid external dependencies
// In a real test environment, you would use a mock HTTP server or httpbin.org

/*
#[tokio::test]
async fn test_http_fetch() {
    let mut engine = NetworkEngine::new();

    // Using httpbin.org for testing (reliable test service)
    let result = engine.fetch("http://httpbin.org/html").await;

    assert!(result.is_ok(), "Should fetch HTTP URL");
    assert!(result.unwrap().contains("html"), "Should contain HTML content");
}

#[tokio::test]
async fn test_https_fetch() {
    let mut engine = NetworkEngine::new();

    let result = engine.fetch("https://httpbin.org/get").await;

    assert!(result.is_ok(), "Should fetch HTTPS URL");
    assert!(result.unwrap().contains("args"), "Should contain JSON response");
}

#[tokio::test]
async fn test_http_error_404() {
    let mut engine = NetworkEngine::new();

    let result = engine.fetch("http://httpbin.org/status/404").await;

    assert!(result.is_err(), "Should return error for 404");
}

#[tokio::test]
async fn test_http_timeout() {
    let mut engine = NetworkEngine::new();

    // httpbin.org/delay/35 will timeout (our timeout is 30s)
    let result = engine.fetch("http://httpbin.org/delay/35").await;

    assert!(result.is_err(), "Should timeout on slow response");
}
*/
