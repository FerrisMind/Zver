use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub enum Resource {
    Css(String, String),    // (url, content)
    Image(String, Vec<u8>), // (url, bytes)
    Script(String, String), // (url, content)
}

#[derive(Debug, Clone)]
pub enum ResourceRequest {
    Css(String),
    Image(String),
    Script(String),
}

pub struct ResourceLoader {
    request_tx: Option<mpsc::UnboundedSender<ResourceRequest>>,
    response_rx: Option<Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Resource>>>>,
    worker_handle: Option<JoinHandle<()>>,
    #[allow(dead_code)]
    client: Option<reqwest::Client>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self {
            request_tx: None,
            response_rx: None,
            worker_handle: None,
            client: None,
        }
    }

    // Инициализация внутри async контекста
    pub async fn init(&mut self) {
        if self.request_tx.is_some() {
            return; // Уже инициализирован
        }

        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        let client = reqwest::Client::builder()
            .user_agent("Zver/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create custom HTTP client: {}", e);
                eprintln!("Falling back to default reqwest client");
                reqwest::Client::new()
            });

        let loader_client = client.clone();

        // Запускаем фоновый worker
        let worker_handle = tokio::spawn(async move {
            Self::worker_loop(request_rx, response_tx, loader_client).await;
        });

        self.request_tx = Some(request_tx);
        self.response_rx = Some(Arc::new(tokio::sync::Mutex::new(response_rx)));
        self.worker_handle = Some(worker_handle);
        self.client = Some(client);
    }

    // Фоновый воркер для загрузки ресурсов
    async fn worker_loop(
        mut request_rx: mpsc::UnboundedReceiver<ResourceRequest>,
        response_tx: mpsc::UnboundedSender<Resource>,
        client: reqwest::Client,
    ) {
        // Пул активных загрузок для параллельной загрузки
        let mut active_tasks: Vec<JoinHandle<Option<Resource>>> = Vec::new();
        const MAX_CONCURRENT: usize = 6; // HTTP/2 рекомендует 6

        loop {
            // Очищаем завершённые задачи
            active_tasks.retain(|t| !t.is_finished());

            // Получаем новые запросы
            if active_tasks.len() < MAX_CONCURRENT {
                match request_rx.try_recv() {
                    Ok(request) => {
                        let client = client.clone();
                        let task =
                            tokio::spawn(
                                async move { Self::fetch_resource(request, client).await },
                            );
                        active_tasks.push(task);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {
                        // Ждём новых запросов
                        if active_tasks.is_empty() {
                            match request_rx.recv().await {
                                Some(request) => {
                                    let client = client.clone();
                                    let task = tokio::spawn(async move {
                                        Self::fetch_resource(request, client).await
                                    });
                                    active_tasks.push(task);
                                }
                                None => break, // Канал закрыт
                            }
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => break,
                }
            }

            // Проверяем завершённые задачи и отправляем результаты
            let mut completed_indices = Vec::new();
            for (idx, task) in active_tasks.iter_mut().enumerate() {
                if task.is_finished() {
                    completed_indices.push(idx);
                }
            }

            for idx in completed_indices.into_iter().rev() {
                if let Ok(Some(resource)) = active_tasks.remove(idx).await {
                    let _ = response_tx.send(resource);
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    // Загрузка одного ресурса
    async fn fetch_resource(request: ResourceRequest, client: reqwest::Client) -> Option<Resource> {
        match request {
            ResourceRequest::Css(url) => match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => match response.text().await {
                    Ok(content) => Some(Resource::Css(url, content)),
                    Err(_) => None,
                },
                _ => None,
            },
            ResourceRequest::Image(url) => match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => match response.bytes().await {
                    Ok(bytes) => Some(Resource::Image(url, bytes.to_vec())),
                    Err(_) => None,
                },
                _ => None,
            },
            ResourceRequest::Script(url) => match client.get(&url).send().await {
                Ok(response) if response.status().is_success() => match response.text().await {
                    Ok(content) => Some(Resource::Script(url, content)),
                    Err(_) => None,
                },
                _ => None,
            },
        }
    }

    // Запрос на загрузку CSS
    pub fn request_css(&self, url: String) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.send(ResourceRequest::Css(url));
        }
    }

    // Запрос на загрузку изображения
    pub fn request_image(&self, url: String) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.send(ResourceRequest::Image(url));
        }
    }

    // Запрос на загрузку скрипта
    pub fn request_script(&self, url: String) {
        if let Some(tx) = &self.request_tx {
            let _ = tx.send(ResourceRequest::Script(url));
        }
    }

    // Получение загруженных ресурсов (неблокирующая)
    pub async fn poll_resources(&self) -> Vec<Resource> {
        let mut resources = Vec::new();

        if let Some(rx) = &self.response_rx {
            let mut rx_guard = rx.lock().await;

            while let Ok(resource) = rx_guard.try_recv() {
                resources.push(resource);
            }
        }

        resources
    }

    // Prefetch список URLs
    pub fn prefetch(&self, urls: &[String], resource_type: &str) {
        for url in urls {
            match resource_type {
                "css" => self.request_css(url.clone()),
                "image" => self.request_image(url.clone()),
                "script" => self.request_script(url.clone()),
                _ => {}
            }
        }
    }
}

impl Drop for ResourceLoader {
    fn drop(&mut self) {
        if let Some(handle) = self.worker_handle.take() {
            handle.abort();
        }
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}
