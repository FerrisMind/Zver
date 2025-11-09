use crate::phosphor_icons::regular;
/// Address bar component with URL input and HTML file scanner
///
/// Implements TRIZ principle of "Dinamichnost" (Dynamicity) with auto-discovery
/// of test HTML files and responsive UI controls
use eframe::egui;
use std::path::PathBuf;

/// Address bar component for URL navigation and DevTools toggle
pub struct AddressBar {
    /// Current URL text in the input field
    pub url_input: String,
    /// List of discovered HTML test files
    pub html_files: Vec<PathBuf>,
    /// Whether DevTools panel is open
    pub devtools_open: bool,
    /// Index of selected HTML file in dropdown (-1 = none)
    pub selected_html_index: Option<usize>,
}

/// Result of rendering the address bar UI
#[derive(Default)]
pub struct AddressBarResult {
    pub load_url: Option<String>,
    pub navigate_back: bool,
    pub navigate_forward: bool,
}

impl Default for AddressBar {
    fn default() -> Self {
        let mut bar = Self {
            url_input: String::new(),
            html_files: Vec::new(),
            devtools_open: false,
            selected_html_index: None,
        };
        bar.scan_html_files();
        bar
    }
}

impl AddressBar {
    /// Creates a new AddressBar and scans for HTML files
    pub fn new() -> Self {
        Self::default()
    }

    /// Scans the tests/ directory for HTML files
    ///
    /// This implements automatic resource discovery, reducing manual configuration
    pub fn scan_html_files(&mut self) {
        self.html_files.clear();

        // Try to find tests directory relative to current working directory
        let test_paths = [
            PathBuf::from("tests"),
            PathBuf::from("../tests"),
            PathBuf::from("../../tests"),
        ];

        for test_path in &test_paths {
            if test_path.exists() && test_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(test_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("html") {
                            self.html_files.push(path);
                        }
                    }
                }
                if !self.html_files.is_empty() {
                    break;
                }
            }
        }

        self.html_files.sort();
    }

    /// Toggles DevTools panel visibility
    pub fn toggle_devtools(&mut self) {
        self.devtools_open = !self.devtools_open;
    }

    /// Sets the URL input from a file path
    ///
    /// # Arguments
    /// * `path` - Path to convert to file:// URL
    pub fn set_url_from_path(&mut self, path: &std::path::Path) {
        if let Some(path_str) = path.to_str() {
            let url = if path_str.starts_with("file://") {
                path_str.to_string()
            } else {
                format!("file://{}", path_str)
            };
            self.url_input = url;
        }
    }

    /// Renders the address bar UI
    ///
    /// # Arguments
    /// * `ui` - egui UI context
    /// * `can_go_back` - whether back navigation is enabled
    /// * `can_go_forward` - whether forward navigation is enabled
    ///
    /// # Returns
    /// Result that includes navigation requests
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        can_go_back: bool,
        can_go_forward: bool,
    ) -> AddressBarResult {
        let mut result = AddressBarResult::default();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_go_back, egui::Button::new(regular::ARROW_LEFT))
                .on_hover_text("Go back")
                .clicked()
            {
                result.navigate_back = true;
            }

            if ui
                .add_enabled(can_go_forward, egui::Button::new(regular::ARROW_RIGHT))
                .on_hover_text("Go forward")
                .clicked()
            {
                result.navigate_forward = true;
            }

            ui.label("URL:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.url_input)
                    .desired_width(ui.available_width() - 256.0)
                    .hint_text("Enter URL or select from dropdown"),
            );

            if response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.url_input.is_empty()
            {
                result.load_url = Some(self.url_input.clone());
            }

            if ui.button(regular::ARROW_RIGHT).on_hover_text("Load URL").clicked() && !self.url_input.is_empty()
            {
                result.load_url = Some(self.url_input.clone());
            }

            if ui
                .button(regular::ARROW_CLOCKWISE)
                .on_hover_text("Reload")
                .clicked()
                && !self.url_input.is_empty()
            {
                result.load_url = Some(self.url_input.clone());
            }

            if !self.html_files.is_empty() {
                        egui::ComboBox::from_label("Test Files")
                            .selected_text(
                                self.selected_html_index
                                    .and_then(|i| self.html_files.get(i))
                                    .and_then(|p| p.file_name())
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Select HTML file"),
                            )
                            .show_ui(ui, |ui| {
                                let html_files = self.html_files.clone();
                                for (index, path) in html_files.iter().enumerate() {
                                    let file_name =
                                        path.file_name().and_then(|n| n.to_str()).unwrap_or("???");

                                    if ui
                                        .selectable_label(
                                            self.selected_html_index == Some(index),
                                            file_name,
                                        )
                                        .clicked()
                                    {
                                        self.selected_html_index = Some(index);
                                        self.set_url_from_path(path);
                                        result.load_url = Some(self.url_input.clone());
                                    }
                                }
                            });
                    }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = if self.devtools_open {
                    egui::Button::new(regular::BUG).fill(egui::Color32::from_rgb(100, 150, 255))
                } else {
                    egui::Button::new(regular::BUG)
                };
                let mut response = ui.add(button);
                if self.devtools_open {
                    response = response.on_hover_text("Toggle DevTools");
                }
                if response.clicked() {
                    self.toggle_devtools();
                }
            });
        });

        result
    }
}
