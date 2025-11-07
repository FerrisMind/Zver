/// DevTools component with Elements, Console, Network, Performance tabs
/// 
/// Implements TRIZ principle of "Vynesenie" (Taking out) where diagnostic tools
/// are separated into an independent, toggleable panel that syncs with active tab
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;
use zver::dom::serialization::serialize_dom;

/// DevTools panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevToolsTab {
    /// DOM tree and structure
    Elements,
    /// JavaScript console logs
    Console,
    /// Network requests (placeholder)
    Network,
    /// Performance metrics
    Performance,
}

impl DevToolsTab {
    /// Returns all available tabs
    pub fn all() -> &'static [DevToolsTab] {
        &[
            DevToolsTab::Elements,
            DevToolsTab::Console,
            DevToolsTab::Network,
            DevToolsTab::Performance,
        ]
    }

    /// Returns the display name for the tab
    pub fn name(&self) -> &'static str {
        match self {
            DevToolsTab::Elements => "Elements",
            DevToolsTab::Console => "Console",
            DevToolsTab::Network => "Network",
            DevToolsTab::Performance => "Performance",
        }
    }
}

/// DevTools panel state
pub struct DevTools {
    /// Currently active tab
    active_tab: DevToolsTab,
    /// Cached HTML serialization from DOM
    cached_html: String,
    /// Cached DOM statistics
    cached_dom_stats: String,
    /// Cached layout statistics
    cached_layout_stats: String,
    /// Cached console logs
    cached_console_logs: Vec<String>,
    /// Whether to show debug overlays in render
    show_debug_overlays: bool,
}

impl Default for DevTools {
    fn default() -> Self {
        Self {
            active_tab: DevToolsTab::Elements,
            cached_html: String::new(),
            cached_dom_stats: "No data".to_string(),
            cached_layout_stats: "No data".to_string(),
            cached_console_logs: vec!["Console initialized".to_string()],
            show_debug_overlays: false,
        }
    }
}

impl DevTools {
    /// Creates a new DevTools instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether debug overlays should be shown
    pub fn show_debug_overlays(&self) -> bool {
        self.show_debug_overlays
    }

    /// Updates DevTools data from the engine
    /// 
    /// # Arguments
    /// * `engine` - The Zver engine to extract data from
    /// * `runtime` - Tokio runtime for async operations
    pub fn update_from_engine(&mut self, engine: &Arc<Zver>, runtime: &Arc<Runtime>) {
        runtime.block_on(async {
            let dom = engine.dom.read().await;
            let layout = engine.layout.read().await;

            // Update HTML serialization
            self.cached_html = serialize_dom(&dom);

            // Update DOM statistics
            self.cached_dom_stats = format!(
                "DOM nodes: {}, Root: {:?}, Parser: scraper",
                dom.nodes.len(),
                dom.root,
            );

            // Update Layout statistics
            let layout_results = layout.get_all_layout_results();
            self.cached_layout_stats = if !layout_results.is_empty() {
                let mut total_width = 0.0f32;
                let mut total_height = 0.0f32;
                for (_node_id, result) in layout_results.iter() {
                    total_width = total_width.max(result.x + result.width);
                    total_height = total_height.max(result.y + result.height);
                }
                format!(
                    "Layout computed: {} elements, Canvas: {:.0}x{:.0}px",
                    layout_results.len(),
                    total_width,
                    total_height
                )
            } else {
                "Layout not computed".to_string()
            };
        });
    }

    /// Renders the DevTools panel
    /// 
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `engine` - Optional reference to current engine for live updates
    /// * `runtime` - Optional runtime for async operations
    pub fn render(&mut self, ui: &mut egui::Ui, engine: Option<&Arc<Zver>>, runtime: Option<&Arc<Runtime>>) {
        // Tab selector
        ui.horizontal(|ui| {
            for tab in DevToolsTab::all() {
                if ui.selectable_label(self.active_tab == *tab, tab.name()).clicked() {
                    self.active_tab = *tab;
                }
            }

            ui.separator();

            // Refresh button
            if ui.button("⟳ Refresh").clicked()
                && let (Some(engine), Some(runtime)) = (engine, runtime) {
                    self.update_from_engine(engine, runtime);
                }

            // Debug overlay toggle
            if self.active_tab == DevToolsTab::Elements {
                ui.separator();
                ui.checkbox(&mut self.show_debug_overlays, "Debug Overlays");
            }
        });

        ui.separator();

        // Render active tab content
        match self.active_tab {
            DevToolsTab::Elements => self.render_elements_tab(ui),
            DevToolsTab::Console => self.render_console_tab(ui),
            DevToolsTab::Network => self.render_network_tab(ui),
            DevToolsTab::Performance => self.render_performance_tab(ui),
        }
    }

    /// Renders the Elements tab with DOM tree
    fn render_elements_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("DOM Structure");
        
        ui.horizontal(|ui| {
            ui.label("Stats:");
            ui.label(&self.cached_dom_stats);
        });

        ui.separator();

        // HTML source in scrollable area
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.cached_html.as_str())
                        .desired_width(f32::INFINITY)
                        .code_editor()
                );
            });
    }

    /// Renders the Console tab with logs
    fn render_console_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Console Logs");
        
        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                self.cached_console_logs.clear();
                self.cached_console_logs.push("Console cleared".to_string());
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, log) in self.cached_console_logs.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", index));
                        ui.label(log);
                    });
                }
            });
    }

    /// Renders the Network tab (placeholder)
    fn render_network_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Network Requests");
        ui.separator();
        ui.label("Network monitoring will be implemented in future updates.");
        ui.label("This panel will show:");
        ui.label("  • HTTP/HTTPS requests");
        ui.label("  • Resource loading times");
        ui.label("  • Cache hits/misses");
        ui.label("  • Request/Response headers");
    }

    /// Renders the Performance tab with metrics
    fn render_performance_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Performance Metrics");
        
        ui.horizontal(|ui| {
            ui.label("Layout:");
            ui.label(&self.cached_layout_stats);
        });

        ui.separator();

        ui.label("Layout Engine: Taffy");
        ui.label("Rendering: WGPU (for full render) / egui Painter (for UI)");
        ui.label("JavaScript: Boa");
        
        ui.separator();
        
        ui.label("Future metrics:");
        ui.label("  • Page load time");
        ui.label("  • Layout computation time");
        ui.label("  • Render time");
        ui.label("  • JavaScript execution time");
    }

    /// Adds a console log message
    /// 
    /// # Arguments
    /// * `message` - The log message to add
    pub fn add_console_log(&mut self, message: String) {
        self.cached_console_logs.push(message);
    }
}
