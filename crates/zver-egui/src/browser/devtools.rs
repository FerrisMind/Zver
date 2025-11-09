/// DevTools component with Elements, Console, Network, Performance tabs
///
/// Implements TRIZ principle of "Vynesenie" (Taking out) where diagnostic tools
/// are separated into an independent, toggleable panel that syncs with active tab
use eframe::egui;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use zver::Zver;
use zver::dom::serialization::serialize_dom;
use zver::dom::{Document, Node};
use zver::js::JSValue;
use zver::network::NetworkLogEntry;

#[derive(Debug, Clone)]
struct DomTreeNode {
    id: usize,
    label: String,
    children: Vec<DomTreeNode>,
}

#[derive(Debug, Clone)]
struct NodeSummary {
    tag_name: Option<String>,
    attributes: Vec<(String, String)>,
    text_preview: Option<String>,
}

/// Console log entry displayed in the DevTools panel.
#[derive(Debug, Clone)]
pub enum ConsoleEntry {
    /// Informational messages such as system notifications.
    Info(String),
    /// Executed command issued by the user.
    Command(String),
    /// Evaluation result emitted by the JS engine.
    Result(String),
    /// Error text produced during JS execution.
    Error(String),
}

impl ConsoleEntry {
    fn prefix(&self) -> &'static str {
        match self {
            ConsoleEntry::Info(_) => "≈",
            ConsoleEntry::Command(_) => ">",
            ConsoleEntry::Result(_) => "<",
            ConsoleEntry::Error(_) => "!",
        }
    }

    fn message(&self) -> &str {
        match self {
            ConsoleEntry::Info(body)
            | ConsoleEntry::Command(body)
            | ConsoleEntry::Result(body)
            | ConsoleEntry::Error(body) => body,
        }
    }

    fn formatted_text(&self) -> String {
        format!("{} {}", self.prefix(), self.message())
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ConsoleEntry::Info(_) => egui::Color32::from_rgb(160, 160, 160),
            ConsoleEntry::Command(_) => egui::Color32::from_rgb(150, 180, 255),
            ConsoleEntry::Result(_) => egui::Color32::from_rgb(200, 255, 200),
            ConsoleEntry::Error(_) => egui::Color32::from_rgb(255, 120, 120),
        }
    }
}

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
    /// Cached DOM tree for the Elements tab
    cached_dom_tree: Option<DomTreeNode>,
    /// Selected DOM node id for highlighting
    selected_node_id: Option<usize>,
    /// Cached summaries for DOM nodes (used for the inspector panel)
    node_snapshots: HashMap<usize, NodeSummary>,
    /// Cached layout statistics
    cached_layout_stats: String,
    /// Cached network log entries
    cached_network_logs: Vec<NetworkLogEntry>,
    /// Cached console logs
    cached_console_logs: Vec<ConsoleEntry>,
    /// Console input buffer
    console_input: String,
    /// Command history for console input
    console_history: Vec<String>,
    /// History cursor index
    console_history_index: Option<usize>,
    /// Whether to show debug overlays in render
    show_debug_overlays: bool,
}

impl Default for DevTools {
    fn default() -> Self {
        Self {
            active_tab: DevToolsTab::Elements,
            cached_html: String::new(),
            cached_dom_stats: "No data".to_string(),
            cached_dom_tree: None,
            selected_node_id: None,
            node_snapshots: HashMap::new(),
            cached_layout_stats: "No data".to_string(),
            cached_network_logs: Vec::new(),
            cached_console_logs: vec![ConsoleEntry::Info("Console initialized".to_string())],
            console_input: String::new(),
            console_history: Vec::new(),
            console_history_index: None,
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
            drop(layout);

            let network = engine.network.read().await;
            self.cached_network_logs = network.logs().to_vec();
            drop(network);

            let mut node_snapshots = HashMap::new();
            self.cached_dom_tree = dom
                .root
                .and_then(|root_id| Self::build_dom_node(&dom, root_id, &mut node_snapshots));
            self.node_snapshots = node_snapshots;

            if self.cached_dom_tree.is_none() {
                self.selected_node_id = None;
            } else if let Some(selected) = self.selected_node_id
                && !dom.nodes.contains_key(&selected)
            {
                self.selected_node_id = None;
            }

            drop(dom);
        });
    }

    /// Renders the DevTools panel
    ///
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `engine` - Optional reference to current engine for live updates
    /// * `runtime` - Optional runtime for async operations
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        engine: Option<&Arc<Zver>>,
        runtime: Option<&Arc<Runtime>>,
    ) {
        // Tab selector
        ui.horizontal(|ui| {
            for tab in DevToolsTab::all() {
                if ui
                    .selectable_label(self.active_tab == *tab, tab.name())
                    .clicked()
                {
                    self.active_tab = *tab;
                }
            }

            ui.separator();

            // Refresh button
            if ui.button("⟳ Refresh").clicked()
                && let (Some(engine), Some(runtime)) = (engine, runtime)
            {
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
            DevToolsTab::Console => self.render_console_tab(ui, engine, runtime),
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

        ui.label("DOM Tree:");
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                if let Some(tree) = self.cached_dom_tree.clone() {
                    self.render_dom_tree_node(ui, &tree, 0);
                } else {
                    ui.label("DOM tree is not available yet.");
                }
            });

        ui.separator();
        ui.heading("Selected node");
        if let Some((node_id, summary)) = self.selected_node_summary() {
            ui.label(format!("Node ID: {}", node_id));
            if let Some(tag) = &summary.tag_name {
                ui.label(format!("Tag: <{}>", tag));
            } else {
                ui.label("Node Type: Text node");
            }
            if !summary.attributes.is_empty() {
                ui.label("Attributes:");
                for (idx, (key, value)) in summary.attributes.iter().enumerate() {
                    if idx >= 6 {
                        ui.label("  ...");
                        break;
                    }
                    ui.horizontal(|ui| {
                        ui.monospace(format!("{}=", key));
                        ui.label(format!("\"{}\"", value));
                    });
                }
            }
            if let Some(text) = &summary.text_preview {
                ui.label(format!(r#"Text: "{}""#, text));
            }
        } else {
            ui.label("Select a DOM node to inspect it.");
        }

        ui.separator();
        ui.label("Serialized HTML:");
        egui::ScrollArea::vertical()
            .max_height(224.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.cached_html)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
            });
    }

    fn render_dom_tree_node(&mut self, ui: &mut egui::Ui, node: &DomTreeNode, depth: usize) {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 12.0);
            let is_selected = self.selected_node_id == Some(node.id);
            
            // Create label with custom styling for selection
            if is_selected {
                // Style visuals to remove stroke for selection
                ui.visuals_mut().selection.stroke = egui::Stroke::NONE;
            }
            
            if ui.selectable_label(is_selected, &node.label).clicked() {
                if is_selected {
                    self.selected_node_id = None;
                } else {
                    self.selected_node_id = Some(node.id);
                }
            }
        });

        for child in &node.children {
            self.render_dom_tree_node(ui, child, depth + 1);
        }
    }

    /// Renders the Console tab with logs
    fn render_console_tab(
        &mut self,
        ui: &mut egui::Ui,
        engine: Option<&Arc<Zver>>,
        runtime: Option<&Arc<Runtime>>,
    ) {
        ui.heading("Console Logs");

        let horizontal_response = ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.console_input)
                    .desired_width(424.0)
                    .hint_text("Enter JS expression"),
            );

            let run_clicked = ui.button("Run").clicked();
            let clear_clicked = ui.button("Clear").clicked();
            (response, run_clicked, clear_clicked)
        });
        let (text_response, run_clicked, clear_clicked) = horizontal_response.inner;

        if clear_clicked {
            self.cached_console_logs.clear();
            self.cached_console_logs
                .push(ConsoleEntry::Info("Console cleared".to_string()));
        }

        let mut should_execute = run_clicked;
        if text_response.has_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.recall_previous_command();
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.recall_next_command();
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                should_execute = true;
            }
        }

        ui.label("Use ↑/↓ to recall history entries.");
        if let Some(history_index) = self.console_history_index
            && let Some(entry) = self.console_history.get(history_index)
        {
            ui.label(format!("History [{}]: {}", history_index + 1, entry));
        }

        if should_execute {
            self.execute_console_command(self.console_input.clone(), engine, runtime);
        }

        ui.separator();

        egui::ScrollArea::vertical()
            .max_height(404.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, entry) in self.cached_console_logs.iter().enumerate() {
                    let entry_text = entry.formatted_text();
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", index));
                        ui.colored_label(entry.color(), entry_text);
                    });
                }
            });
    }

    /// Renders the Network tab (minimal log view)
    fn render_network_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Network Requests");
        ui.separator();

        if self.cached_network_logs.is_empty() {
            ui.label("No network activity recorded yet.");
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(404.0)
            .show(ui, |ui| {
                for entry in self.cached_network_logs.iter().rev() {
                    ui.horizontal_wrapped(|ui| {
                        let status_color = if entry.status.starts_with("OK") {
                            egui::Color32::from_rgb(120, 255, 180)
                        } else {
                            egui::Color32::from_rgb(255, 150, 150)
                        };
                        ui.colored_label(status_color, format!("[{}]", entry.status));
                        ui.label(format!("({}) {}", entry.source, entry.url));
                    });
                    ui.separator();
                }
            });
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

    fn recall_previous_command(&mut self) {
        if self.console_history.is_empty() {
            return;
        }

        let history_len = self.console_history.len();
        let next_index = match self.console_history_index {
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => 0,
            None => history_len.saturating_sub(1),
        };

        if let Some(entry) = self.console_history.get(next_index) {
            self.console_input = entry.clone();
            self.console_history_index = Some(next_index);
        }
    }

    fn recall_next_command(&mut self) {
        if let Some(idx) = self.console_history_index {
            if idx + 1 < self.console_history.len() {
                let next_index = idx + 1;
                if let Some(entry) = self.console_history.get(next_index) {
                    self.console_input = entry.clone();
                    self.console_history_index = Some(next_index);
                }
            } else {
                self.console_history_index = None;
                self.console_input.clear();
            }
        }
    }

    fn execute_console_command(
        &mut self,
        input: String,
        engine: Option<&Arc<Zver>>,
        runtime: Option<&Arc<Runtime>>,
    ) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }

        let command = trimmed.to_string();
        self.console_history.push(command.clone());
        if self.console_history.len() > 200 {
            self.console_history.remove(0);
        }
        self.console_history_index = None;
        self.console_input.clear();

        self.cached_console_logs
            .push(ConsoleEntry::Command(command.clone()));

        if let (Some(engine), Some(runtime)) = (engine, runtime) {
            let result = runtime.block_on(async {
                let mut js_engine = engine.js.write().await;
                js_engine.execute(command.as_str())
            });

            match result {
                Ok(value) => {
                    self.cached_console_logs
                        .push(ConsoleEntry::Result(Self::format_js_value(&value)));
                }
                Err(err) => {
                    self.cached_console_logs
                        .push(ConsoleEntry::Error(format!("Error: {}", err)));
                }
            }
        } else {
            self.cached_console_logs.push(ConsoleEntry::Error(
                "JavaScript engine not available".to_string(),
            ));
        }
    }

    fn format_js_value(value: &JSValue) -> String {
        match value {
            JSValue::Undefined => "undefined".to_string(),
            JSValue::Number(num) => num.to_string(),
            JSValue::String(text) => text.clone(),
            JSValue::Boolean(flag) => flag.to_string(),
            JSValue::Object(_) => "[object Object]".to_string(),
        }
    }

    fn build_dom_node(
        document: &Document,
        node_id: usize,
        snapshots: &mut HashMap<usize, NodeSummary>,
    ) -> Option<DomTreeNode> {
        let node = document.nodes.get(&node_id)?;
        snapshots.insert(node_id, Self::build_node_summary(node));
        let mut children = Vec::new();
        for &child_id in &node.children {
            if let Some(child_node) = Self::build_dom_node(document, child_id, snapshots) {
                children.push(child_node);
            }
        }
        Some(DomTreeNode {
            id: node_id,
            label: Self::format_node_label(node),
            children,
        })
    }

    fn build_node_summary(node: &Node) -> NodeSummary {
        let attributes = Self::collect_sorted_attributes(node);
        let mut preview = None;
        if let Some(text) = &node.text_content {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                const MAX_PREVIEW_CHARS: usize = 80;
                let mut preview_builder = String::new();
                let mut truncated = false;
                for (idx, ch) in trimmed.chars().enumerate() {
                    if idx >= MAX_PREVIEW_CHARS {
                        truncated = true;
                        break;
                    }
                    preview_builder.push(ch);
                }
                if truncated {
                    preview_builder.push_str("...");
                }
                preview = Some(preview_builder);
            }
        }

        NodeSummary {
            tag_name: node.tag_name.clone(),
            attributes,
            text_preview: preview,
        }
    }

    fn format_node_label(node: &Node) -> String {
        let mut label = format!("[{}] ", node.id);
        if let Some(tag) = node.tag_name.as_deref() {
            label.push_str(&format!("<{}>", tag));
        } else {
            label.push_str("#text");
        }

        let attributes = Self::collect_sorted_attributes(node);
        if !attributes.is_empty() {
            const MAX_ATTRS_DISPLAY: usize = 5;
            let mut displayed = Vec::new();
            for (key, value) in attributes.iter().take(MAX_ATTRS_DISPLAY) {
                displayed.push(format!(r#"{}="{}""#, key, value));
            }
            if attributes.len() > MAX_ATTRS_DISPLAY {
                displayed.push("...".to_string());
            }
            if !displayed.is_empty() {
                label.push(' ');
                label.push_str(&displayed.join(" "));
            }
        }

        if node.tag_name.is_none()
            && let Some(text) = &node.text_content
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let mut preview = String::new();
                let mut truncated = false;
                for (idx, ch) in trimmed.chars().enumerate() {
                    if idx >= 30 {
                        truncated = true;
                        break;
                    }
                    preview.push(ch);
                }
                label.push_str(&format!(
                    r#" "{}{}""#,
                    preview,
                    if truncated { "..." } else { "" }
                ));
            }
        }

        label
    }

    fn collect_sorted_attributes(node: &Node) -> Vec<(String, String)> {
        let mut keys: Vec<_> = node.attributes.keys().cloned().collect();
        keys.sort();
        keys.into_iter()
            .filter_map(|key| {
                node.attributes
                    .get(&key)
                    .map(|value| (key.clone(), value.clone()))
            })
            .collect()
    }

    fn selected_node_summary(&self) -> Option<(usize, &NodeSummary)> {
        self.selected_node_id.and_then(|node_id| {
            self.node_snapshots
                .get(&node_id)
                .map(|summary| (node_id, summary))
        })
    }

    pub fn selected_node_id(&self) -> Option<usize> {
        self.selected_node_id
    }

    /// Adds a console entry to the DevTools output.
    ///
    /// # Arguments
    /// * `entry` - The typed console entry (Info, Command, Result, Error)
    pub fn add_console_log(&mut self, entry: ConsoleEntry) {
        self.cached_console_logs.push(entry);
    }
}
