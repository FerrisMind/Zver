use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NetworkEngine {
    cache: HashMap<String, String>,
    client: reqwest::Client,
}

impl NetworkEngine {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Zver/0.1 (reqwest)")
            .http2_prior_knowledge()
            .build()
            .expect("failed to build reqwest client");

        Self {
            cache: HashMap::new(),
            client,
        }
    }

    pub fn clear_cache_for_url(&mut self, url: &str) {
        self.cache.remove(url);
    }

    pub async fn fetch(&mut self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(cached) = self.cache.get(url) {
            return Ok(cached.clone());
        }

        let content = if url.starts_with("http://") || url.starts_with("https://") {
            self.fetch_http_https(url).await?
        } else if url.starts_with("file://") {
            self.fetch_file(url).await?
        } else {
            self.fetch_file(&format!("file://{}", url)).await?
        };

        self.cache.insert(url.to_string(), content.clone());
        Ok(content)
    }

    // Prefetch ресурсов асинхронно
    pub async fn prefetch_resources(&self, urls: Vec<String>) -> Vec<Result<String, Box<dyn std::error::Error>>> {
        let futures = urls.into_iter().map(|url| {
            let client = &self.client;
            async move {
                let result: Result<String, Box<dyn std::error::Error>> = if url.starts_with("http://") || url.starts_with("https://") {
                    client.get(&url).send().await?.text().await.map_err(|e| e.into())
                } else {
                    // Для файлов используем tokio::fs
                    tokio::fs::read_to_string(url.strip_prefix("file://").unwrap_or(&url)).await.map_err(|e| e.into())
                };
                result
            }
        });

        futures_util::future::join_all(futures).await
    }

    async fn fetch_http_https(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        let text = response.text().await?;
        Ok(text)
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

