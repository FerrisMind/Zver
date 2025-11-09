/// Tab component with isolated engine instance
///
/// Implements TRIZ principle of "Dinamichnost" (Dynamicity) where each tab
/// maintains independent state and can be added/removed dynamically
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;

/// Status of a browser tab
#[derive(Debug, Clone, PartialEq)]
pub enum TabStatus {
    /// Tab is idle, waiting for user action
    Idle,
    /// Tab is currently loading content
    Loading,
    /// Tab has successfully loaded content
    Loaded,
    /// Tab encountered an error
    Error(String),
}

/// Represents a single browser tab with isolated engine instance
pub struct Tab {
    /// Unique identifier for the tab
    #[allow(dead_code)]
    pub id: usize,
    /// Display title extracted from URL or page
    pub title: String,
    /// Current URL loaded in the tab
    pub url: String,
    /// Isolated Zver engine instance for this tab
    pub engine: Arc<Zver>,
    /// Current status of the tab
    pub status: TabStatus,
    /// Back navigation history stack
    back_stack: Vec<String>,
    /// Forward navigation history stack
    forward_stack: Vec<String>,
}

impl Tab {
    /// Выполняет JS в контексте движка вкладки.
    #[allow(dead_code)]
    pub fn eval_js(&self, code: &str) -> Result<zver::js::JSValue, String> {
        self.engine.eval_js(code)
    }
    /// Creates a new tab with a fresh engine instance
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the tab
    /// * `runtime` - Tokio runtime for async operations
    pub fn new(id: usize, _runtime: &Arc<Runtime>) -> Self {
        Self {
            id,
            title: format!("New Tab {}", id),
            url: String::new(),
            engine: Arc::new(Zver::new()),
            status: TabStatus::Idle,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        }
    }

    /// Loads a URL in this tab
    ///
    /// # Arguments
    /// * `url` - The URL to load
    /// * `runtime` - Tokio runtime for blocking async operations
    pub fn load_url(&mut self, url: String, runtime: &Arc<Runtime>, track_history: bool) {
        if track_history && !self.url.is_empty() {
            self.back_stack.push(self.url.clone());
            self.forward_stack.clear();
        }
        self.url = url.clone();
        self.status = TabStatus::Loading;
        self.title = Self::extract_title_from_url(&url);

        let engine = Arc::clone(&self.engine);
        let result = runtime.block_on(async move { engine.load_url(&url).await });

        match result {
            Ok(_) => {
                self.status = TabStatus::Loaded;
            }
            Err(e) => {
                self.status = TabStatus::Error(format!("{}", e));
            }
        }
    }

    /// Extracts a display title from a URL
    ///
    /// # Arguments
    /// * `url` - The URL to extract title from
    ///
    /// # Returns
    /// A human-readable title string
    fn extract_title_from_url(url: &str) -> String {
        if url.is_empty() {
            return "New Tab".to_string();
        }

        // Extract filename from file:// URLs
        if let Some(path) = url.strip_prefix("file://")
            && let Some(filename) = path.split(['/', '\\']).next_back()
        {
            return filename.to_string();
        }

        // Extract domain from http(s):// URLs
        if let Some(rest) = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            && let Some(domain) = rest.split('/').next()
        {
            return domain.to_string();
        }

        // Fallback to truncated URL
        if url.len() > 30 {
            format!("{}...", &url[..27])
        } else {
            url.to_string()
        }
    }

    /// Reloads the current URL, clearing cache
    ///
    /// # Arguments
    /// * `runtime` - Tokio runtime for async operations
    pub fn reload(&mut self, runtime: &Arc<Runtime>) {
        if self.url.is_empty() {
            return;
        }

        let url = self.url.clone();
        self.status = TabStatus::Loading;

        // Clear cache before reload
        let engine = Arc::clone(&self.engine);
        runtime.block_on(async {
            let mut network = engine.network.write().await;
            network.clear_cache_for_url(&url);
        });
        self.load_url(url, runtime, false);
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    pub fn go_back(&mut self, runtime: &Arc<Runtime>) {
        if let Some(previous) = self.back_stack.pop() {
            if !self.url.is_empty() {
                self.forward_stack.push(self.url.clone());
            }
            self.load_url(previous, runtime, false);
        }
    }

    pub fn go_forward(&mut self, runtime: &Arc<Runtime>) {
        if let Some(next) = self.forward_stack.pop() {
            if !self.url.is_empty() {
                self.back_stack.push(self.url.clone());
            }
            self.load_url(next, runtime, false);
        }
    }
}

/// Manages multiple browser tabs
///
/// Implements TRIZ principle of "Obedinenie" (Merging) where tabs share
/// a common interface but maintain isolated state
pub struct TabManager {
    tabs: Vec<Tab>,
    active_tab_index: usize,
    next_tab_id: usize,
    runtime: Arc<Runtime>,
}

impl TabManager {
    /// Maximum number of tabs allowed
    pub const MAX_TABS: usize = 5;

    /// Creates a new TabManager with one initial tab
    ///
    /// # Arguments
    /// * `runtime` - Tokio runtime for async operations
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let initial_tab = Tab::new(0, &runtime);
        Self {
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 1,
            runtime,
        }
    }

    /// Returns the number of tabs
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Returns the active tab index
    pub fn active_index(&self) -> usize {
        self.active_tab_index
    }

    /// Returns a reference to all tabs
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Returns a reference to the active tab
    pub fn get_active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Returns a mutable reference to the active tab
    pub fn get_active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Выполняет JS в активной вкладке.
    ///
    /// Если активной вкладки нет — возвращает Err("No active tab").
    #[allow(dead_code)]
    pub fn eval_in_active_tab(&self, code: &str) -> Result<zver::js::JSValue, String> {
        if let Some(tab) = self.get_active_tab() {
            tab.eval_js(code)
        } else {
            Err("No active tab".to_string())
        }
    }

    /// Sets the active tab by index
    ///
    /// # Arguments
    /// * `index` - The index of the tab to activate
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
        }
    }

    /// Adds a new tab
    ///
    /// # Returns
    /// `true` if the tab was added, `false` if at maximum capacity
    pub fn add_tab(&mut self) -> bool {
        if self.tabs.len() >= Self::MAX_TABS {
            return false;
        }

        let new_tab = Tab::new(self.next_tab_id, &self.runtime);
        self.next_tab_id += 1;
        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;
        true
    }

    /// Closes a tab by index
    ///
    /// # Arguments
    /// * `index` - The index of the tab to close
    ///
    /// # Returns
    /// `true` if the tab was closed, `false` if it's the last tab
    pub fn close_tab(&mut self, index: usize) -> bool {
        // Don't close the last tab
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return false;
        }

        self.tabs.remove(index);

        // Adjust active tab index
        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index <= self.active_tab_index && self.active_tab_index > 0 {
            self.active_tab_index -= 1;
        }

        true
    }

    /// Loads a URL in the active tab
    ///
    /// # Arguments
    /// * `url` - The URL to load
    pub fn load_url_in_active_tab(&mut self, url: String) {
        let runtime = Arc::clone(&self.runtime);
        if let Some(tab) = self.get_active_tab_mut() {
            tab.load_url(url, &runtime, true);
        }
    }

    /// Reloads the active tab
    pub fn reload_active_tab(&mut self) {
        let runtime = Arc::clone(&self.runtime);
        if let Some(tab) = self.get_active_tab_mut() {
            tab.reload(&runtime);
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.get_active_tab()
            .map(|tab| tab.can_go_back())
            .unwrap_or(false)
    }

    pub fn can_go_forward(&self) -> bool {
        self.get_active_tab()
            .map(|tab| tab.can_go_forward())
            .unwrap_or(false)
    }

    pub fn go_back_active_tab(&mut self) {
        let runtime = Arc::clone(&self.runtime);
        if let Some(tab) = self.get_active_tab_mut() {
            tab.go_back(&runtime);
        }
    }

    pub fn go_forward_active_tab(&mut self) {
        let runtime = Arc::clone(&self.runtime);
        if let Some(tab) = self.get_active_tab_mut() {
            tab.go_forward(&runtime);
        }
    }
}
