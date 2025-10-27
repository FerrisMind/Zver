use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone)]
pub struct NetworkEngine {
    cache: HashMap<String, String>,
}

impl NetworkEngine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub async fn fetch(&mut self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(cached) = self.cache.get(url) {
            return Ok(cached.clone());
        }

        let content = if url.starts_with("http://") {
            self.fetch_http(url).await?
        } else if url.starts_with("https://") {
            return Err("HTTPS не поддерживается в примитивном клиенте".into());
        } else if url.starts_with("file://") {
            self.fetch_file(url).await?
        } else {
            self.fetch_file(&format!("file://{url}")).await?
        };

        self.cache.insert(url.to_string(), content.clone());
        Ok(content)
    }

    async fn fetch_http(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = url.strip_prefix("http://").unwrap_or(url);
        let (host_port, path) = url.split_once('/').unwrap_or((url, "/"));
        let (host, port) = host_port
            .split_once(':')
            .map(|(host, port)| (host, port.parse().unwrap_or(80)))
            .unwrap_or((host_port, 80));

        let mut stream = TcpStream::connect((host, port)).await?;

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;

        if let Some(body_start) = response.find("\r\n\r\n") {
            Ok(response[body_start + 4..].to_string())
        } else {
            Ok(response)
        }
    }

    async fn fetch_file(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = url.strip_prefix("file://").unwrap_or(url);
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }
}

impl Default for NetworkEngine {
    fn default() -> Self {
        Self::new()
    }
}

