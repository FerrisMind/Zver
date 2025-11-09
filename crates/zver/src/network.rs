use std::collections::HashMap;
use std::time::Duration;

/// Simple HTTP log entry for DevTools
#[derive(Debug, Clone)]
pub struct NetworkLogEntry {
    pub url: String,
    pub status: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct NetworkEngine {
    cache: HashMap<String, String>,
    client: reqwest::Client,
    logs: Vec<NetworkLogEntry>,
}

impl NetworkEngine {
    const MAX_LOG_ENTRIES: usize = 256;

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
            .unwrap_or_else(|e| {
                eprintln!("Failed to build reqwest client with full config: {}", e);
                eprintln!("Falling back to default client configuration");
                reqwest::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .expect("Failed to build even basic HTTP client - this is a critical error")
            });

        Self {
            cache: HashMap::new(),
            client,
            logs: Vec::new(),
        }
    }

    fn record_network_event(&mut self, url: &str, status: String, source: String) {
        if self.logs.len() >= Self::MAX_LOG_ENTRIES {
            self.logs.remove(0);
        }
        self.logs.push(NetworkLogEntry {
            url: url.to_string(),
            status,
            source,
        });
    }

    pub fn logs(&self) -> &[NetworkLogEntry] {
        &self.logs
    }

    fn categorize_resource(url: &str) -> &'static str {
        let cleaned = url
            .split(|c| ['?', '#'].contains(&c))
            .next()
            .unwrap_or(url)
            .to_ascii_lowercase();

        if cleaned.ends_with(".css") {
            "CSS"
        } else if cleaned.ends_with(".js") {
            "JS"
        } else {
            "HTML"
        }
    }

    pub fn clear_cache_for_url(&mut self, url: &str) {
        self.cache.remove(url);
    }

    pub async fn fetch(&mut self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let source_label = Self::categorize_resource(url).to_string();

        if let Some(cached) = self.cache.get(url).cloned() {
            self.record_network_event(url, "OK (cache)".to_string(), source_label.clone());
            return Ok(cached);
        }

        let result = if url.starts_with("http://") || url.starts_with("https://") {
            self.fetch_http_https(url).await
        } else if url.starts_with("file://") {
            self.fetch_file(url).await
        } else {
            self.fetch_file(&format!("file://{}", url)).await
        };

        match &result {
            Ok(content) => {
                self.cache.insert(url.to_string(), content.clone());
                self.record_network_event(url, "OK".to_string(), source_label.clone());
            }
            Err(err) => {
                self.record_network_event(url, format!("ERR: {}", err), source_label.clone());
            }
        }

        result
    }

    // Prefetch ресурсов асинхронно с использованием пула соединений
    pub async fn prefetch_resources(
        &self,
        urls: Vec<String>,
    ) -> Vec<Result<String, Box<dyn std::error::Error>>> {
        use futures_util::future::join_all;

        let futures = urls.into_iter().map(|url| {
            let client = self.client.clone();
            async move {
                let result: Result<String, Box<dyn std::error::Error>> =
                    if url.starts_with("http://") || url.starts_with("https://") {
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
        let response = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
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
