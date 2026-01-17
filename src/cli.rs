use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::schema;
use crate::types::{ProcessingOptions, DEFAULT_K_ANONYMITY};

/// Privacy-preserving metadata extraction from data files
#[derive(Parser, Debug)]
#[command(name = "ert-manifest")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan a data file and extract metadata
    Scan {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output JSON file path (stdout if not specified)
        #[arg(short, long)]
        out: Option<PathBuf>,

        /// K-anonymity threshold
        #[arg(short, long, default_value_t = DEFAULT_K_ANONYMITY)]
        k: u64,

        /// Bucket counts instead of exact values
        #[arg(long, default_value_t = true)]
        bucket_counts: bool,

        /// Use exact counts (requires --relaxed)
        #[arg(long, default_value_t = false)]
        exact_counts: bool,

        /// Use exact median (requires --relaxed)
        #[arg(long, default_value_t = false)]
        exact_median: bool,

        /// Compute file hash (SHA-256)
        #[arg(long, default_value_t = true)]
        hash_file: bool,

        /// Enable relaxed mode (allows exact counts/median)
        #[arg(long, default_value_t = false)]
        relaxed: bool,
    },

    /// Launch the GUI
    Gui,
}

/// GUI Application state
#[derive(Debug, Clone, PartialEq)]
pub enum GuiState {
    Ready,
    Processing,
    Done,
    Error(String),
}

impl Default for GuiState {
    fn default() -> Self {
        GuiState::Ready
    }
}

/// GUI Application
pub struct GuiApp {
    state: GuiState,
    dropped_file: Option<PathBuf>,
    result: Option<String>,
    warnings: Vec<String>,
    options: ProcessingOptions,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            state: GuiState::Ready,
            dropped_file: None,
            result: None,
            warnings: Vec::new(),
            options: ProcessingOptions::default(),
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(path) = i.raw.dropped_files[0].path.clone() {
                    self.dropped_file = Some(path);
                    self.state = GuiState::Processing;
                }
            }
        });

        // Process file if needed
        if self.state == GuiState::Processing {
            if let Some(ref path) = self.dropped_file {
                match schema::extract_schema(path, self.options.clone()) {
                    Ok(extraction_result) => {
                        let manifest = &extraction_result.manifest;
                        self.warnings = manifest.warnings.clone();
                        for sheet in &manifest.sheets {
                            self.warnings.extend(sheet.warnings.clone());
                            for col in &sheet.columns {
                                self.warnings.extend(col.warnings.clone());
                            }
                        }

                        // Write sidekick file if recoding was done
                        if let Some(ref sidekick_content) = extraction_result.recode_sidekick {
                            let sidekick_path = path.with_extension("recode.txt");
                            if let Err(e) = std::fs::write(&sidekick_path, sidekick_content) {
                                self.warnings.push(format!("Failed to write recode file: {}", e));
                            } else {
                                self.warnings.push(format!(
                                    "Recode mapping saved to: {}",
                                    sidekick_path.display()
                                ));
                            }
                        }

                        match crate::output::to_json_string(manifest) {
                            Ok(json) => {
                                self.result = Some(json);
                                self.state = GuiState::Done;
                            }
                            Err(e) => {
                                self.state = GuiState::Error(e.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        self.state = GuiState::Error(e.to_string());
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ert-manifest v0.1.0");
            ui.add_space(10.0);

            match &self.state {
                GuiState::Ready => {
                    self.show_ready_state(ui);
                }
                GuiState::Processing => {
                    ui.spinner();
                    ui.label("Processing file...");
                }
                GuiState::Done => {
                    self.show_done_state(ui, ctx);
                }
                GuiState::Error(msg) => {
                    let msg = msg.clone();
                    self.show_error_state(ui, msg);
                }
            }
        });
    }
}

impl GuiApp {
    fn show_ready_state(&mut self, ui: &mut egui::Ui) {
        // Drag and drop zone
        let drop_zone = egui::Frame::none()
            .fill(egui::Color32::from_gray(40))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_gray(100)))
            .rounding(10.0)
            .inner_margin(40.0);

        drop_zone.show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label("Drag and drop a data file here");
                ui.label("(CSV, TSV, Excel)");
                ui.add_space(20.0);
                ui.label("or");
                ui.add_space(10.0);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Data files", &["csv", "tsv", "xlsx", "xls"])
                        .pick_file()
                    {
                        self.dropped_file = Some(path);
                        self.state = GuiState::Processing;
                    }
                }
            });
        });

        ui.add_space(20.0);

        // Options
        ui.collapsing("Options", |ui| {
            ui.horizontal(|ui| {
                ui.label("K-anonymity:");
                let mut k = self.options.k_anonymity as i32;
                if ui.add(egui::Slider::new(&mut k, 1..=20)).changed() {
                    self.options.k_anonymity = k as u64;
                }
            });

            ui.checkbox(&mut self.options.bucket_counts, "Bucket counts");
            ui.checkbox(&mut self.options.hash_file, "Compute file hash");

            ui.add_enabled_ui(self.options.relaxed, |ui| {
                ui.checkbox(&mut self.options.exact_counts, "Exact counts");
                ui.checkbox(&mut self.options.exact_median, "Exact median");
            });

            ui.checkbox(&mut self.options.relaxed, "Relaxed mode");
        });
    }

    fn show_done_state(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Show warnings if any
        let warnings_count = self.warnings.len();
        if warnings_count > 0 {
            let warnings_clone = self.warnings.clone();
            ui.collapsing(format!("Warnings ({})", warnings_count), |ui| {
                for warning in &warnings_clone {
                    ui.colored_label(egui::Color32::YELLOW, warning);
                }
            });
            ui.add_space(10.0);
        }

        // Show result
        let json_clone = self.result.clone();
        if let Some(ref json) = json_clone {
            ui.label("Manifest generated successfully!");
            ui.add_space(10.0);

            let mut should_reset = false;
            let mut should_copy = false;
            let mut save_path: Option<PathBuf> = None;

            ui.horizontal(|ui| {
                if ui.button("Copy to clipboard").clicked() {
                    should_copy = true;
                }

                if ui.button("Save to file...").clicked() {
                    save_path = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .save_file();
                }

                if ui.button("New file").clicked() {
                    should_reset = true;
                }
            });

            if should_copy {
                ctx.copy_text(json.clone());
            }

            if let Some(path) = save_path {
                if let Err(e) = std::fs::write(&path, json) {
                    self.state = GuiState::Error(e.to_string());
                    return;
                }
            }

            if should_reset {
                self.reset();
                return;
            }

            ui.add_space(10.0);

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    let mut text = json.as_str();
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
        }
    }

    fn show_error_state(&mut self, ui: &mut egui::Ui, msg: String) {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", msg));
        ui.add_space(20.0);
        if ui.button("Try again").clicked() {
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.state = GuiState::Ready;
        self.dropped_file = None;
        self.result = None;
        self.warnings.clear();
    }
}
