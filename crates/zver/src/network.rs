use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NetworkEngine {
    cache: HashMap<String, String>,
    client: reqwest::Client,
}

impl NetworkEngine {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Zver/0.1.0 (Rust Browser Engine)")
            // Полноценная настройка HTTP/2
            .http2_prior_knowledge()
            .http2_adaptive_window(true)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(90))
            .http2_keep_alive_while_idle(true)
            // TLS настройки
            .use_rustls_tls()
            .https_only(false)
            .tls_sni(true)
            // Connection pooling
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            // Таймауты
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            // Редиректы
            .redirect(reqwest::redirect::Policy::limited(10))
            // Компрессия включена автоматически при наличии features
            // Производительность
            .tcp_nodelay(true)
            .tcp_keepalive(Duration::from_secs(60))
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

    // Prefetch ресурсов асинхронно с использованием пула соединений
    pub async fn prefetch_resources(&self, urls: Vec<String>) -> Vec<Result<String, Box<dyn std::error::Error>>> {
        use futures_util::future::join_all;
        
        let futures = urls.into_iter().map(|url| {
            let client = self.client.clone();
            async move {
                let result: Result<String, Box<dyn std::error::Error>> = if url.starts_with("http://") || url.starts_with("https://") {
                    match client.get(&url).send().await {
                        Ok(response) => response.text().await.map_err(|e| e.into()),
                        Err(e) => Err(e.into()),
                    }
                } else {
                    // Для файлов используем tokio::fs
                    tokio::fs::read_to_string(url.strip_prefix("file://").unwrap_or(&url))
                        .await
                        .map_err(|e| e.into())
                };
                result
            }
        });

        join_all(futures).await
    }

    async fn fetch_http_https(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .get(url)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
            .send()
            .await?;
        
        // Проверяем статус
        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP error: {} for {}", status, url).into());
        }
        
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

