use eframe::egui;
use file_compressor::{
    compress_directory, compress_file, decompress_file, format_ratio, format_size, verify_zst,
    CompressOptions, DecompressOptions,
};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([550.0, 450.0])
            .with_min_inner_size([450.0, 350.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "File Compressor",
        options,
        Box::new(|_cc| Ok(Box::new(CompressorApp::default()))),
    )
}

#[derive(Debug, Clone, PartialEq)]
enum Operation {
    Compress,
    Decompress,
    Verify,
}

#[derive(Debug, Clone)]
struct TaskResult {
    #[allow(dead_code)]
    success: bool,
    message: String,
    details: Vec<String>,
}

struct CompressorApp {
    selected_files: Vec<PathBuf>,
    compression_level: i32,
    operation: Operation,
    force_overwrite: bool,
    parallel: bool,
    status_message: String,
    is_processing: bool,
    result_receiver: Option<Receiver<TaskResult>>,
    progress: f32,
    show_details: bool,
    last_details: Vec<String>,
}

impl Default for CompressorApp {
    fn default() -> Self {
        Self {
            selected_files: Vec::new(),
            compression_level: 3,
            operation: Operation::Compress,
            force_overwrite: false,
            parallel: false,
            status_message: "Trascina i file qui o usa il pulsante per selezionarli".to_string(),
            is_processing: false,
            result_receiver: None,
            progress: 0.0,
            show_details: false,
            last_details: Vec::new(),
        }
    }
}

impl CompressorApp {
    fn process_files(&mut self) {
        if self.selected_files.is_empty() {
            self.status_message = "Nessun file selezionato!".to_string();
            return;
        }

        let (tx, rx): (Sender<TaskResult>, Receiver<TaskResult>) = channel();
        self.result_receiver = Some(rx);
        self.is_processing = true;
        self.progress = 0.0;

        let files = self.selected_files.clone();
        let level = self.compression_level;
        let force = self.force_overwrite;
        let parallel = self.parallel;
        let operation = self.operation.clone();

        thread::spawn(move || {
            let result = match operation {
                Operation::Compress => compress_files(&files, level, force, parallel),
                Operation::Decompress => decompress_files(&files, force),
                Operation::Verify => verify_files(&files),
            };

            let _ = tx.send(result);
        });
    }
}

impl eframe::App for CompressorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Gestione drag and drop
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        if !self.selected_files.contains(path) {
                            self.selected_files.push(path.clone());
                        }
                    }
                }
                self.status_message = format!("{} file selezionati", self.selected_files.len());
            }
        });

        // Controlla se l'operazione è completata
        if let Some(ref rx) = self.result_receiver {
            if let Ok(result) = rx.try_recv() {
                self.is_processing = false;
                self.progress = 1.0;
                self.status_message = result.message;
                self.last_details = result.details;
                self.result_receiver = None;
            }
        }

        // Pannello centrale
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🗜️ File Compressor");
            ui.add_space(10.0);

            // Selezione operazione
            ui.horizontal(|ui| {
                ui.label("Operazione:");
                ui.selectable_value(&mut self.operation, Operation::Compress, "Comprimi");
                ui.selectable_value(&mut self.operation, Operation::Decompress, "Decomprimi");
                ui.selectable_value(&mut self.operation, Operation::Verify, "Verifica");
            });

            ui.add_space(10.0);

            // Opzioni per compressione
            if matches!(self.operation, Operation::Compress) {
                ui.horizontal(|ui| {
                    ui.label("Livello compressione:");
                    ui.add(egui::Slider::new(&mut self.compression_level, 1..=21));
                    ui.label(compression_level_hint(self.compression_level));
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.parallel, "Compressione parallela (multi-core)");
                });
            }

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.force_overwrite, "Sovrascrivi file esistenti");
            });

            ui.add_space(10.0);

            // Area drag and drop
            let drop_area = ui.group(|ui| {
                ui.set_min_size(egui::vec2(ui.available_width(), 80.0));
                ui.centered_and_justified(|ui| {
                    if self.selected_files.is_empty() {
                        ui.label("📁 Trascina i file qui");
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label(format!("📁 {} elementi selezionati", self.selected_files.len()));
                        });
                    }
                });
            });

            // Evidenzia l'area durante il drag
            if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
                ui.painter().rect_stroke(
                    drop_area.response.rect,
                    4.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100)),
                );
            }

            ui.add_space(5.0);

            // Lista file selezionati (scrollabile)
            if !self.selected_files.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(80.0)
                    .show(ui, |ui| {
                        let mut to_remove = Vec::new();
                        for (i, file) in self.selected_files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let file_name = file
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| file.to_string_lossy().to_string());

                                let icon = if file.is_dir() { "📁" } else { "📄" };

                                if ui.small_button("❌").clicked() {
                                    to_remove.push(i);
                                }
                                ui.label(format!("{} {}", icon, file_name));
                            });
                        }
                        for i in to_remove.into_iter().rev() {
                            self.selected_files.remove(i);
                        }
                    });
            }

            ui.add_space(10.0);

            // Pulsanti azione
            ui.horizontal(|ui| {
                if ui.button("📂 Seleziona file").clicked() && !self.is_processing {
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        for path in paths {
                            if !self.selected_files.contains(&path) {
                                self.selected_files.push(path);
                            }
                        }
                        self.status_message =
                            format!("{} file selezionati", self.selected_files.len());
                    }
                }

                if ui.button("📁 Seleziona cartella").clicked() && !self.is_processing {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        if !self.selected_files.contains(&path) {
                            self.selected_files.push(path);
                        }
                        self.status_message =
                            format!("{} elementi selezionati", self.selected_files.len());
                    }
                }

                if ui.button("🗑️ Pulisci").clicked() && !self.is_processing {
                    self.selected_files.clear();
                    self.last_details.clear();
                    self.status_message =
                        "Trascina i file qui o usa il pulsante per selezionarli".to_string();
                }
            });

            ui.add_space(10.0);

            // Pulsante esegui
            let button_text = match self.operation {
                Operation::Compress => "🗜️ Comprimi",
                Operation::Decompress => "📦 Decomprimi",
                Operation::Verify => "✅ Verifica",
            };

            ui.add_enabled_ui(!self.is_processing && !self.selected_files.is_empty(), |ui| {
                if ui.button(button_text).clicked() {
                    self.process_files();
                }
            });

            // Progress bar
            if self.is_processing {
                ui.add_space(5.0);
                ui.add(egui::ProgressBar::new(self.progress).animate(true));
                ctx.request_repaint();
            }

            ui.add_space(10.0);

            // Status message
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.label(&self.status_message);

                // Mostra dettagli se disponibili
                if !self.last_details.is_empty() {
                    ui.add_space(5.0);
                    if ui.small_button(if self.show_details { "▼ Nascondi dettagli" } else { "▶ Mostra dettagli" }).clicked() {
                        self.show_details = !self.show_details;
                    }

                    if self.show_details {
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for detail in &self.last_details {
                                    ui.label(detail);
                                }
                            });
                    }
                }
            });
        });
    }
}

/// Restituisce un hint sul livello di compressione
fn compression_level_hint(level: i32) -> &'static str {
    match level {
        1..=3 => "(veloce)",
        4..=9 => "(bilanciato)",
        10..=15 => "(lento)",
        16..=21 => "(molto lento)",
        _ => "",
    }
}

/// Comprime i file selezionati
fn compress_files(files: &[PathBuf], level: i32, force: bool, parallel: bool) -> TaskResult {
    let mut success_count = 0;
    let mut error_count = 0;
    let mut details = Vec::new();
    let mut total_original = 0u64;
    let mut total_compressed = 0u64;

    let options = CompressOptions::new(level)
        .with_force(force)
        .with_parallel(parallel);

    for file in files {
        if file.is_dir() {
            match compress_directory(file, &options) {
                Ok(result) => {
                    success_count += 1;
                    total_original += result.input_size;
                    total_compressed += result.output_size;
                    details.push(format!(
                        "✅ {:?} -> {} ({})",
                        file.file_name().unwrap_or_default(),
                        format_size(result.output_size),
                        format_ratio(result.input_size, result.output_size)
                    ));
                }
                Err(e) => {
                    error_count += 1;
                    details.push(format!("❌ {:?}: {}", file.file_name().unwrap_or_default(), e));
                }
            }
        } else {
            match compress_file(file, &options) {
                Ok(result) => {
                    success_count += 1;
                    total_original += result.input_size;
                    total_compressed += result.output_size;
                    details.push(format!(
                        "✅ {:?} -> {} ({})",
                        file.file_name().unwrap_or_default(),
                        format_size(result.output_size),
                        format_ratio(result.input_size, result.output_size)
                    ));
                }
                Err(e) => {
                    error_count += 1;
                    details.push(format!("❌ {:?}: {}", file.file_name().unwrap_or_default(), e));
                }
            }
        }
    }

    let summary = if error_count == 0 {
        format!(
            "✅ Compressi {} elementi: {} -> {} ({})",
            success_count,
            format_size(total_original),
            format_size(total_compressed),
            format_ratio(total_original, total_compressed)
        )
    } else {
        format!(
            "⚠️ Compressi {} elementi, {} errori",
            success_count, error_count
        )
    };

    TaskResult {
        success: error_count == 0,
        message: summary,
        details,
    }
}

/// Decomprime i file selezionati
fn decompress_files(files: &[PathBuf], force: bool) -> TaskResult {
    let mut success_count = 0;
    let mut error_count = 0;
    let mut details = Vec::new();

    let options = DecompressOptions::new().with_force(force);

    for file in files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "zst" {
            error_count += 1;
            details.push(format!("⏭️ {:?}: non è un file .zst", file.file_name().unwrap_or_default()));
            continue;
        }

        match decompress_file(file, &options) {
            Ok(result) => {
                success_count += 1;
                details.push(format!(
                    "✅ {:?} -> {}",
                    file.file_name().unwrap_or_default(),
                    format_size(result.output_size)
                ));
            }
            Err(e) => {
                error_count += 1;
                details.push(format!("❌ {:?}: {}", file.file_name().unwrap_or_default(), e));
            }
        }
    }

    let summary = if error_count == 0 {
        format!("✅ Decompressi {} file con successo!", success_count)
    } else {
        format!(
            "⚠️ Decompressi {} file, {} errori/saltati",
            success_count, error_count
        )
    };

    TaskResult {
        success: error_count == 0,
        message: summary,
        details,
    }
}

/// Verifica l'integrità dei file
fn verify_files(files: &[PathBuf]) -> TaskResult {
    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut skipped_count = 0;
    let mut details = Vec::new();

    for file in files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "zst" {
            skipped_count += 1;
            details.push(format!("⏭️ {:?}: non è un file .zst", file.file_name().unwrap_or_default()));
            continue;
        }

        match verify_zst(file, None) {
            Ok(result) => {
                valid_count += 1;
                details.push(format!(
                    "✅ {:?}: valido ({} -> {})",
                    file.file_name().unwrap_or_default(),
                    format_size(result.compressed_size),
                    format_size(result.decompressed_size)
                ));
            }
            Err(e) => {
                invalid_count += 1;
                details.push(format!("❌ {:?}: {}", file.file_name().unwrap_or_default(), e));
            }
        }
    }

    let summary = if invalid_count == 0 && skipped_count == 0 {
        format!("✅ {} file validi!", valid_count)
    } else if invalid_count == 0 {
        format!("✅ {} file validi, {} saltati", valid_count, skipped_count)
    } else {
        format!("⚠️ {} validi, {} corrotti, {} saltati", valid_count, invalid_count, skipped_count)
    };

    TaskResult {
        success: invalid_count == 0,
        message: summary,
        details,
    }
}
