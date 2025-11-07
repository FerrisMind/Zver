use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod egui_integration;
mod browser;

use browser::{TabManager, AddressBar, DevTools, RenderView};

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Zver Browser",
        native_options,
        Box::new(|_cc| Ok(Box::<ZverBrowser>::default())),
    )
}

/// Zver Browser application - implements TRIZ browser architecture
/// 
/// Uses modular components (Tab, AddressBar, DevTools, RenderView) to create
/// a browser-like interface with tab management and developer tools
struct ZverBrowser {
    runtime: Arc<Runtime>,
    tab_manager: TabManager,
    address_bar: AddressBar,
    devtools: DevTools,
}

impl Default for ZverBrowser {
    fn default() -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
        let tab_manager = TabManager::new(Arc::clone(&runtime));
        let address_bar = AddressBar::new();
        let devtools = DevTools::new();

        Self {
            runtime,
            tab_manager,
            address_bar,
            devtools,
        }
    }
}

impl eframe::App for ZverBrowser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel: Tab bar
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            self.render_tab_bar(ui);
        });

        // Top panel: Address bar
        egui::TopBottomPanel::top("address_bar_panel").show(ctx, |ui| {
            if let Some(url) = self.address_bar.render(ui) {
                let url_for_log = url.clone();
                self.tab_manager.load_url_in_active_tab(url.clone());
                self.address_bar.url_input = url;
                
                // Update DevTools after loading
                if let Some(tab) = self.tab_manager.get_active_tab() {
                    self.devtools.update_from_engine(&tab.engine, &self.runtime);
                    self.devtools.add_console_log(format!("Loaded: {}", url_for_log));
                }
            }
        });

        // Bottom panel: DevTools (if open)
        if self.address_bar.devtools_open {
            egui::TopBottomPanel::bottom("devtools_panel")
                .min_height(300.0)
                .show(ctx, |ui| {
                    let engine = self.tab_manager.get_active_tab().map(|t| &t.engine);
                    self.devtools.render(ui, engine, Some(&self.runtime));
                });
        }

        // Central panel: Render view
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.tab_manager.get_active_tab() {
                ui.heading(format!("🌐 {}", tab.title));
                ui.label(format!("Status: {:?}", tab.status));
                ui.separator();

                RenderView::render(
                    ui,
                    &tab.engine,
                    &self.runtime,
                    self.devtools.show_debug_overlays(),
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No active tab");
                });
            }
        });
    }
}

impl ZverBrowser {
    /// Renders the tab bar with tab buttons and controls
    fn render_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Collect tab info to avoid borrowing issues
            let tabs_info: Vec<_> = self.tab_manager.tabs()
                .iter()
                .map(|tab| {
                    if tab.title.len() > 20 {
                        format!("{}...", &tab.title[..17])
                    } else {
                        tab.title.clone()
                    }
                })
                .collect();
            
            let active_index = self.tab_manager.active_index();
            let tab_count = self.tab_manager.tab_count();

            // Render tab buttons
            for (index, tab_text) in tabs_info.iter().enumerate() {
                let is_active = index == active_index;

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Tab button
                        if ui.selectable_label(is_active, tab_text).clicked() {
                            self.tab_manager.set_active_tab(index);
                            
                            // Sync DevTools with new active tab
                            if let Some(tab) = self.tab_manager.get_active_tab() {
                                self.devtools.update_from_engine(&tab.engine, &self.runtime);
                                self.address_bar.url_input = tab.url.clone();
                            }
                        }

                        // Close button (only if more than 1 tab)
                        if tab_count > 1
                            && ui.small_button("✖").clicked() {
                                self.tab_manager.close_tab(index);
                                
                                // Update DevTools after closing
                                if let Some(tab) = self.tab_manager.get_active_tab() {
                                    self.devtools.update_from_engine(&tab.engine, &self.runtime);
                                    self.address_bar.url_input = tab.url.clone();
                                }
                            }
                    });
                });
            }

            // New tab button (if under limit)
            if tab_count < TabManager::MAX_TABS
                && ui.button("➕ New Tab").clicked() {
                    self.tab_manager.add_tab();
                }

            ui.separator();

            // Reload button
            if ui.button("⟳").on_hover_text("Reload active tab").clicked() {
                self.tab_manager.reload_active_tab();
                
                // Update DevTools after reload
                if let Some(tab) = self.tab_manager.get_active_tab() {
                    self.devtools.update_from_engine(&tab.engine, &self.runtime);
                    self.devtools.add_console_log("Page reloaded".to_string());
                }
            }

            // Tab counter
            ui.label(format!(
                "Tabs: {}/{}",
                tab_count,
                TabManager::MAX_TABS
            ));
        });
    }
}
