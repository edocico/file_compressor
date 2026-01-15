use eframe::egui;
use file_compressor::{
    compress_directory, compress_file, decompress_file, format_ratio, format_size, verify_zst,
    CompressOptions, DecompressOptions,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

// ============================================================================
// Localizzazione
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Language {
    Italian,
    English,
}

impl Language {
    fn detect() -> Self {
        if let Some(locale) = sys_locale::get_locale() {
            let locale_lower = locale.to_lowercase();
            if locale_lower.starts_with("it") {
                Language::Italian
            } else {
                Language::English
            }
        } else {
            Language::English
        }
    }
}

struct Strings {
    // UI labels
    operation: &'static str,
    compress: &'static str,
    decompress: &'static str,
    verify: &'static str,
    compression_level: &'static str,
    parallel_compression: &'static str,
    overwrite_existing: &'static str,
    destination: &'static str,
    same_folder: &'static str,
    choose: &'static str,
    drag_files_here: &'static str,
    elements_selected: &'static str,
    files_selected: &'static str,
    select_files: &'static str,
    select_folder: &'static str,
    clear: &'static str,
    no_file_selected: &'static str,
    drag_or_select: &'static str,

    // Compression level hints
    fast: &'static str,
    balanced: &'static str,
    slow: &'static str,
    very_slow: &'static str,

    // Action buttons
    compress_btn: &'static str,
    decompress_btn: &'static str,
    verify_btn: &'static str,

    // Details
    hide_details: &'static str,
    show_details: &'static str,

    // Results
    compressed_elements: &'static str,
    compressed_with_errors: &'static str,
    decompressed_success: &'static str,
    decompressed_with_errors: &'static str,
    files_valid: &'static str,
    files_valid_skipped: &'static str,
    files_valid_corrupt_skipped: &'static str,
    not_zst_file: &'static str,
    valid: &'static str,
}

const STRINGS_IT: Strings = Strings {
    operation: "Operazione:",
    compress: "Comprimi",
    decompress: "Decomprimi",
    verify: "Verifica",
    compression_level: "Livello compressione:",
    parallel_compression: "Compressione parallela (multi-core)",
    overwrite_existing: "Sovrascrivi file esistenti",
    destination: "Destinazione:",
    same_folder: "(stessa cartella del file)",
    choose: "Scegli...",
    drag_files_here: "Trascina i file qui",
    elements_selected: "elementi selezionati",
    files_selected: "file selezionati",
    select_files: "Seleziona file",
    select_folder: "Seleziona cartella",
    clear: "Pulisci",
    no_file_selected: "Nessun file selezionato!",
    drag_or_select: "Trascina i file qui o usa il pulsante per selezionarli",
    fast: "(veloce)",
    balanced: "(bilanciato)",
    slow: "(lento)",
    very_slow: "(molto lento)",
    compress_btn: "Comprimi",
    decompress_btn: "Decomprimi",
    verify_btn: "Verifica",
    hide_details: "Nascondi dettagli",
    show_details: "Mostra dettagli",
    compressed_elements: "Compressi {} elementi: {} -> {} ({})",
    compressed_with_errors: "Compressi {} elementi, {} errori",
    decompressed_success: "Decompressi {} file con successo!",
    decompressed_with_errors: "Decompressi {} file, {} errori/saltati",
    files_valid: "{} file validi!",
    files_valid_skipped: "{} file validi, {} saltati",
    files_valid_corrupt_skipped: "{} validi, {} corrotti, {} saltati",
    not_zst_file: "non è un file .zst",
    valid: "valido",
};

const STRINGS_EN: Strings = Strings {
    operation: "Operation:",
    compress: "Compress",
    decompress: "Decompress",
    verify: "Verify",
    compression_level: "Compression level:",
    parallel_compression: "Parallel compression (multi-core)",
    overwrite_existing: "Overwrite existing files",
    destination: "Destination:",
    same_folder: "(same folder as file)",
    choose: "Choose...",
    drag_files_here: "Drag files here",
    elements_selected: "elements selected",
    files_selected: "files selected",
    select_files: "Select files",
    select_folder: "Select folder",
    clear: "Clear",
    no_file_selected: "No file selected!",
    drag_or_select: "Drag files here or use the button to select them",
    fast: "(fast)",
    balanced: "(balanced)",
    slow: "(slow)",
    very_slow: "(very slow)",
    compress_btn: "Compress",
    decompress_btn: "Decompress",
    verify_btn: "Verify",
    hide_details: "Hide details",
    show_details: "Show details",
    compressed_elements: "Compressed {} elements: {} -> {} ({})",
    compressed_with_errors: "Compressed {} elements, {} errors",
    decompressed_success: "Decompressed {} files successfully!",
    decompressed_with_errors: "Decompressed {} files, {} errors/skipped",
    files_valid: "{} valid files!",
    files_valid_skipped: "{} valid files, {} skipped",
    files_valid_corrupt_skipped: "{} valid, {} corrupt, {} skipped",
    not_zst_file: "not a .zst file",
    valid: "valid",
};

fn get_strings(lang: Language) -> &'static Strings {
    match lang {
        Language::Italian => &STRINGS_IT,
        Language::English => &STRINGS_EN,
    }
}

// ============================================================================
// Application
// ============================================================================

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 500.0])
            .with_drag_and_drop(true),
        // Theme is set via ctx.set_visuals() in the callback below
        ..Default::default()
    };

    eframe::run_native(
        "File Compressor",
        options,
        Box::new(|cc| {
            // Force dark mode and configure custom style
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            setup_custom_style(&cc.egui_ctx);
            Ok(Box::new(CompressorApp::default()))
        }),
    )
}

/// Configura uno stile personalizzato professionale con tema scuro
fn setup_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Forza visuals scuri come base
    style.visuals = egui::Visuals::dark();
    let visuals = &mut style.visuals;

    // Colore primario blu professionale (più luminoso per tema scuro)
    let primary_color = egui::Color32::from_rgb(80, 140, 220);
    let primary_hover = egui::Color32::from_rgb(100, 160, 240);

    // Sfondo scuro per finestra principale (leggermente più chiaro per CentralPanel)
    visuals.window_fill = egui::Color32::from_rgb(35, 38, 45);
    visuals.panel_fill = egui::Color32::from_rgb(40, 44, 52); // CentralPanel - slightly lighter
    visuals.faint_bg_color = egui::Color32::from_rgb(50, 55, 65);
    visuals.extreme_bg_color = egui::Color32::from_rgb(25, 28, 32);

    // Widget colors per tema scuro
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(55, 60, 70);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 75, 85));
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 205, 215));

    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 55, 65);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(45, 50, 58);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(65, 70, 80));
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 185, 195));

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 70, 85);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(55, 65, 78);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, primary_color);
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 225, 235));

    visuals.widgets.active.bg_fill = primary_color;
    visuals.widgets.active.weak_bg_fill = primary_hover;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, primary_color);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    // Colori testo per leggibilità
    visuals.override_text_color = Some(egui::Color32::from_rgb(230, 235, 245));

    // Selezione
    visuals.selection.bg_fill = primary_color.linear_multiply(0.5);
    visuals.selection.stroke = egui::Stroke::new(1.0, primary_color);

    // Arrotondamento moderno
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    style.visuals.window_rounding = egui::Rounding::same(12.0);

    // Spacing migliorato
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16.0);

    ctx.set_style(style);
}

// Colori per i pannelli (per distinguere visivamente)
const TOP_PANEL_COLOR: egui::Color32 = egui::Color32::from_rgb(28, 31, 38); // Darker for controls
const BOTTOM_PANEL_COLOR: egui::Color32 = egui::Color32::from_rgb(28, 31, 38); // Same as top
const CENTRAL_PANEL_COLOR: egui::Color32 = egui::Color32::from_rgb(40, 44, 52); // Lighter for file list

#[derive(Debug, Clone, PartialEq)]
enum Operation {
    Compress,
    Decompress,
    Verify,
}

#[derive(Debug, Clone)]
enum TaskMessage {
    Progress(f32),
    Result(TaskResult),
}

#[derive(Debug, Clone)]
struct TaskResult {
    #[allow(dead_code)]
    success: bool,
    message: String,
    details: Vec<String>,
}

struct TaskContext {
    progress_tx: Sender<TaskMessage>,
    cancel_flag: Arc<AtomicU64>,
}

struct CompressorApp {
    selected_files: Vec<PathBuf>,
    compression_level: i32,
    operation: Operation,
    force_overwrite: bool,
    parallel: bool,
    output_directory: Option<PathBuf>,
    status_message: String,
    is_processing: bool,
    result_receiver: Option<Receiver<TaskMessage>>,
    cancel_flag: Arc<AtomicU64>,
    progress: f32,
    show_details: bool,
    last_details: Vec<String>,
    language: Language,
}

impl Default for CompressorApp {
    fn default() -> Self {
        let lang = Language::detect();
        let strings = get_strings(lang);
        Self {
            selected_files: Vec::new(),
            compression_level: 3,
            operation: Operation::Compress,
            force_overwrite: false,
            parallel: false,
            output_directory: None,
            status_message: strings.drag_or_select.to_string(),
            is_processing: false,
            result_receiver: None,
            cancel_flag: Arc::new(AtomicU64::new(0)),
            progress: 0.0,
            show_details: false,
            last_details: Vec::new(),
            language: lang,
        }
    }
}

impl CompressorApp {
    fn strings(&self) -> &'static Strings {
        get_strings(self.language)
    }

    fn process_files(&mut self) {
        let strings = self.strings();
        if self.selected_files.is_empty() {
            self.status_message = strings.no_file_selected.to_string();
            return;
        }

        let (tx, rx): (Sender<TaskMessage>, Receiver<TaskMessage>) = channel();
        self.result_receiver = Some(rx);
        self.is_processing = true;
        self.progress = 0.0;
        self.cancel_flag.store(0, Ordering::Relaxed);

        let files = self.selected_files.clone();
        let level = self.compression_level;
        let force = self.force_overwrite;
        let parallel = self.parallel;
        let operation = self.operation.clone();
        let output_dir = self.output_directory.clone();
        let lang = self.language;
        let cancel_flag = Arc::clone(&self.cancel_flag);

        thread::spawn(move || {
            let ctx = TaskContext {
                progress_tx: tx.clone(),
                cancel_flag,
            };

            let result = match operation {
                Operation::Compress => compress_files(
                    &files,
                    level,
                    force,
                    parallel,
                    output_dir.as_deref(),
                    lang,
                    &ctx,
                ),
                Operation::Decompress => {
                    decompress_files(&files, force, output_dir.as_deref(), lang, &ctx)
                }
                Operation::Verify => verify_files(&files, lang, &ctx),
            };

            let _ = tx.send(TaskMessage::Result(result));
        });
    }

    fn cancel_operation(&mut self) {
        if self.is_processing {
            self.cancel_flag.store(1, Ordering::Relaxed);
            self.status_message = "Annullamento in corso...".to_string();
        }
    }
}

impl eframe::App for CompressorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ensure dark visuals are applied every frame (macOS fix)
        ctx.set_visuals(egui::Visuals::dark());

        let strings = self.strings();

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
                self.status_message =
                    format!("{} {}", self.selected_files.len(), strings.files_selected);
            }
        });

        // Controlla messaggi dal worker thread
        let mut should_clear_receiver = false;
        if let Some(ref rx) = self.result_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    TaskMessage::Progress(p) => {
                        self.progress = p;
                    }
                    TaskMessage::Result(result) => {
                        self.is_processing = false;
                        self.progress = 1.0;
                        self.status_message = result.message;
                        self.last_details = result.details;
                        should_clear_receiver = true;
                    }
                }
            }
        }
        if should_clear_receiver {
            self.result_receiver = None;
        }

        // ============================================================
        // TOP PANEL - Controls (darker background)
        // ============================================================
        egui::TopBottomPanel::top("control_panel")
            .frame(
                egui::Frame::none()
                    .fill(TOP_PANEL_COLOR)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                // Header con titolo
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("File Compressor")
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(230, 235, 245)),
                    );
                    ui.add_space(8.0);
                });

                ui.separator();
                ui.add_space(8.0);

                // Selezione operazione
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(45, 50, 60))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(strings.operation)
                                    .size(14.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(200, 210, 225)),
                            );
                            ui.add_space(8.0);
                            ui.selectable_value(
                                &mut self.operation,
                                Operation::Compress,
                                egui::RichText::new(strings.compress)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(200, 210, 225)),
                            );
                            ui.selectable_value(
                                &mut self.operation,
                                Operation::Decompress,
                                egui::RichText::new(strings.decompress)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(200, 210, 225)),
                            );
                            ui.selectable_value(
                                &mut self.operation,
                                Operation::Verify,
                                egui::RichText::new(strings.verify)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(200, 210, 225)),
                            );
                        });
                    });

                ui.add_space(10.0);

                // Opzioni per compressione
                if matches!(self.operation, Operation::Compress) {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(45, 50, 60))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(strings.compression_level)
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(200, 210, 225)),
                                );
                                ui.add(
                                    egui::Slider::new(&mut self.compression_level, 1..=21)
                                        .text("")
                                        .show_value(true),
                                );
                                ui.label(
                                    egui::RichText::new(compression_level_hint(
                                        self.compression_level,
                                        self.language,
                                    ))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(140, 150, 170)),
                                );
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.parallel, "");
                                ui.label(
                                    egui::RichText::new(strings.parallel_compression)
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(200, 210, 225)),
                                );
                            });
                        });
                    ui.add_space(8.0);
                }

                // Overwrite e destination in una riga
                ui.horizontal(|ui| {
                    // Overwrite checkbox
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(45, 50, 60))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.force_overwrite, "");
                                ui.label(
                                    egui::RichText::new(strings.overwrite_existing)
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(200, 210, 225)),
                                );
                            });
                        });

                    // Destination (se non Verify)
                    if !matches!(self.operation, Operation::Verify) {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(45, 50, 60))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(strings.destination)
                                            .size(13.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(200, 210, 225)),
                                    );
                                    if let Some(ref dir) = self.output_directory {
                                        let dir_name = dir
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| dir.to_string_lossy().to_string());
                                        ui.label(
                                            egui::RichText::new(&dir_name)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(100, 180, 255)),
                                        );
                                        if ui.small_button("X").clicked() {
                                            self.output_directory = None;
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new(strings.same_folder)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(140, 150, 170)),
                                        );
                                    }
                                    if ui
                                        .add_enabled(
                                            !self.is_processing,
                                            egui::Button::new(
                                                egui::RichText::new(strings.choose).size(12.0),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            self.output_directory = Some(path);
                                        }
                                    }
                                });
                            });
                    }
                });

                ui.add_space(10.0);

                // Pulsanti selezione file
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    if ui
                        .add_enabled(
                            !self.is_processing,
                            egui::Button::new(
                                egui::RichText::new(format!("+ {}", strings.select_files))
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(220, 225, 235)),
                            )
                            .min_size(egui::vec2(130.0, 30.0))
                            .fill(egui::Color32::from_rgb(60, 100, 160)),
                        )
                        .clicked()
                    {
                        if let Some(paths) = rfd::FileDialog::new().pick_files() {
                            for path in paths {
                                if !self.selected_files.contains(&path) {
                                    self.selected_files.push(path);
                                }
                            }
                            self.status_message =
                                format!("{} {}", self.selected_files.len(), strings.files_selected);
                        }
                    }

                    if ui
                        .add_enabled(
                            !self.is_processing,
                            egui::Button::new(
                                egui::RichText::new(format!("+ {}", strings.select_folder))
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(220, 225, 235)),
                            )
                            .min_size(egui::vec2(140.0, 30.0))
                            .fill(egui::Color32::from_rgb(60, 100, 160)),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            if !self.selected_files.contains(&path) {
                                self.selected_files.push(path);
                            }
                            self.status_message = format!(
                                "{} {}",
                                self.selected_files.len(),
                                strings.elements_selected
                            );
                        }
                    }

                    if ui
                        .add_enabled(
                            !self.is_processing && !self.selected_files.is_empty(),
                            egui::Button::new(
                                egui::RichText::new(strings.clear)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(220, 225, 235)),
                            )
                            .min_size(egui::vec2(80.0, 30.0))
                            .fill(egui::Color32::from_rgb(120, 70, 70)),
                        )
                        .clicked()
                    {
                        self.selected_files.clear();
                        self.last_details.clear();
                        self.status_message = strings.drag_or_select.to_string();
                    }

                    ui.add_space(20.0);

                    // Pulsante esegui principale
                    let (button_text, button_color) = match self.operation {
                        Operation::Compress => (
                            strings.compress_btn.to_string(),
                            egui::Color32::from_rgb(60, 120, 200),
                        ),
                        Operation::Decompress => (
                            strings.decompress_btn.to_string(),
                            egui::Color32::from_rgb(50, 160, 80),
                        ),
                        Operation::Verify => (
                            strings.verify_btn.to_string(),
                            egui::Color32::from_rgb(100, 110, 130),
                        ),
                    };

                    let enabled = !self.is_processing && !self.selected_files.is_empty();

                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(
                                egui::RichText::new(&button_text)
                                    .size(14.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .min_size(egui::vec2(120.0, 30.0))
                            .fill(if enabled {
                                button_color
                            } else {
                                egui::Color32::from_rgb(80, 85, 95)
                            })
                            .rounding(egui::Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        self.process_files();
                    }
                });

                ui.add_space(4.0);
            });

        // ============================================================
        // BOTTOM PANEL - Status Bar (darker background)
        // ============================================================
        egui::TopBottomPanel::bottom("status_panel")
            .frame(
                egui::Frame::none()
                    .fill(BOTTOM_PANEL_COLOR)
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                // Progress bar (se in elaborazione)
                if self.is_processing {
                    ui.horizontal(|ui| {
                        let progress_bar = egui::ProgressBar::new(self.progress)
                            .show_percentage()
                            .desired_height(20.0)
                            .fill(egui::Color32::from_rgb(80, 140, 220));

                        ui.add_sized([ui.available_width() - 100.0, 20.0], progress_bar);
                        ui.add_space(8.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Cancel")
                                        .size(12.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .min_size(egui::vec2(80.0, 24.0))
                                .fill(egui::Color32::from_rgb(180, 60, 70))
                                .rounding(egui::Rounding::same(4.0)),
                            )
                            .clicked()
                        {
                            self.cancel_operation();
                        }
                    });
                    ctx.request_repaint();
                    ui.add_space(8.0);
                }

                // Status message
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 190, 210)),
                    );
                });

                // Dettagli (se disponibili)
                if !self.last_details.is_empty() {
                    ui.add_space(6.0);
                    let details_btn_text = if self.show_details {
                        format!("v {}", strings.hide_details)
                    } else {
                        format!("> {}", strings.show_details)
                    };

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(details_btn_text)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(160, 170, 190)),
                            )
                            .small()
                            .fill(egui::Color32::from_rgb(45, 50, 60)),
                        )
                        .clicked()
                    {
                        self.show_details = !self.show_details;
                    }

                    if self.show_details {
                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(35, 40, 48))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 60, 70)))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        for detail in &self.last_details {
                                            ui.label(
                                                egui::RichText::new(detail)
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(160, 170, 185)),
                                            );
                                            ui.add_space(2.0);
                                        }
                                    });
                            });
                    }
                }
            });

        // ============================================================
        // CENTRAL PANEL - File List (lighter background)
        // ============================================================
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(CENTRAL_PANEL_COLOR)
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                // Drag and drop indicator
                let is_dragging = ctx.input(|i| !i.raw.hovered_files.is_empty());

                if self.selected_files.is_empty() {
                    // Empty state - drag zone
                    let drop_bg = if is_dragging {
                        egui::Color32::from_rgb(50, 70, 100)
                    } else {
                        egui::Color32::from_rgb(50, 55, 65)
                    };

                    let drop_stroke = if is_dragging {
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 140, 220))
                    } else {
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(70, 80, 95))
                    };

                    egui::Frame::none()
                        .fill(drop_bg)
                        .stroke(drop_stroke)
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(30.0))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(
                                ui.available_width(),
                                ui.available_height() - 20.0,
                            ));
                            ui.centered_and_justified(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(
                                        egui::RichText::new(if is_dragging {
                                            "Drop files here"
                                        } else {
                                            strings.drag_files_here
                                        })
                                        .size(18.0)
                                        .color(
                                            if is_dragging {
                                                egui::Color32::from_rgb(100, 180, 255)
                                            } else {
                                                egui::Color32::from_rgb(140, 150, 170)
                                            },
                                        ),
                                    );
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new(strings.drag_or_select)
                                            .size(13.0)
                                            .color(egui::Color32::from_rgb(110, 120, 140)),
                                    );
                                });
                            });
                        });
                } else {
                    // File list with header
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}",
                                self.selected_files.len(),
                                strings.elements_selected
                            ))
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::from_rgb(200, 210, 225)),
                        );
                    });

                    ui.add_space(10.0);

                    // File list in ScrollArea with Grid layout
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 40, 48))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 60, 70)))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            // Header row
                            ui.horizontal(|ui| {
                                ui.add_space(30.0);
                                ui.label(
                                    egui::RichText::new("Filename")
                                        .size(12.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(140, 150, 170)),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(10.0);
                                        ui.label(
                                            egui::RichText::new("Size")
                                                .size(12.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(140, 150, 170)),
                                        );
                                    },
                                );
                            });
                            ui.separator();

                            // Scrollable file list
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let mut to_remove = Vec::new();

                                    for (i, file) in self.selected_files.iter().enumerate() {
                                        let stripe_bg = if i % 2 == 0 {
                                            egui::Color32::from_rgb(38, 43, 52)
                                        } else {
                                            egui::Color32::from_rgb(42, 47, 56)
                                        };

                                        egui::Frame::none()
                                            .fill(stripe_bg)
                                            .rounding(egui::Rounding::same(4.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    // Remove button
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                egui::RichText::new("x")
                                                                    .size(10.0)
                                                                    .color(
                                                                        egui::Color32::from_rgb(
                                                                            180, 100, 100,
                                                                        ),
                                                                    ),
                                                            )
                                                            .small()
                                                            .fill(egui::Color32::from_rgb(
                                                                60, 45, 50,
                                                            )),
                                                        )
                                                        .clicked()
                                                    {
                                                        to_remove.push(i);
                                                    }

                                                    ui.add_space(6.0);

                                                    // File icon and name
                                                    let file_name = file
                                                        .file_name()
                                                        .map(|n| n.to_string_lossy().to_string())
                                                        .unwrap_or_else(|| {
                                                            file.to_string_lossy().to_string()
                                                        });

                                                    let icon =
                                                        if file.is_dir() { "[D]" } else { "[F]" };
                                                    let icon_color = if file.is_dir() {
                                                        egui::Color32::from_rgb(100, 180, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(180, 190, 210)
                                                    };

                                                    ui.label(
                                                        egui::RichText::new(icon)
                                                            .size(11.0)
                                                            .color(icon_color),
                                                    );
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        egui::RichText::new(&file_name)
                                                            .size(12.0)
                                                            .color(egui::Color32::from_rgb(
                                                                200, 210, 225,
                                                            )),
                                                    );

                                                    // File size on the right
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let size_str = if let Ok(meta) =
                                                                std::fs::metadata(file)
                                                            {
                                                                format_size(meta.len())
                                                            } else {
                                                                "-".to_string()
                                                            };
                                                            ui.label(
                                                                egui::RichText::new(size_str)
                                                                    .size(11.0)
                                                                    .color(
                                                                        egui::Color32::from_rgb(
                                                                            140, 150, 170,
                                                                        ),
                                                                    ),
                                                            );
                                                        },
                                                    );
                                                });
                                            });

                                        ui.add_space(2.0);
                                    }

                                    // Remove files marked for deletion
                                    for i in to_remove.into_iter().rev() {
                                        self.selected_files.remove(i);
                                    }
                                });
                        });

                    // Drag indicator when files exist
                    if is_dragging {
                        ui.add_space(10.0);
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("Drop to add more files")
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                            );
                        });
                    }
                }
            });
    }
}

/// Restituisce un hint sul livello di compressione
fn compression_level_hint(level: i32, lang: Language) -> &'static str {
    let strings = get_strings(lang);
    match level {
        1..=3 => strings.fast,
        4..=9 => strings.balanced,
        10..=15 => strings.slow,
        16..=21 => strings.very_slow,
        _ => "",
    }
}

/// Comprime i file selezionati
fn compress_files(
    files: &[PathBuf],
    level: i32,
    force: bool,
    parallel: bool,
    output_dir: Option<&Path>,
    lang: Language,
    ctx: &TaskContext,
) -> TaskResult {
    let strings = get_strings(lang);
    let mut success_count = 0;
    let mut error_count = 0;
    let mut details = Vec::new();
    let mut total_original = 0u64;
    let mut total_compressed = 0u64;

    let mut options = CompressOptions::new(level)
        .with_force(force)
        .with_parallel(parallel);

    if let Some(dir) = output_dir {
        options = options.with_output_path(dir);
    }

    let total_files = files.len();
    for (idx, file) in files.iter().enumerate() {
        // Controlla flag cancellazione
        if ctx.cancel_flag.load(Ordering::Relaxed) != 0 {
            details.push("❌ Operazione annullata dall'utente".to_string());
            break;
        }

        // Invia progress
        let progress = (idx as f32) / (total_files as f32);
        let _ = ctx.progress_tx.send(TaskMessage::Progress(progress));
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
                    details.push(format!(
                        "❌ {:?}: {}",
                        file.file_name().unwrap_or_default(),
                        e
                    ));
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
                    details.push(format!(
                        "❌ {:?}: {}",
                        file.file_name().unwrap_or_default(),
                        e
                    ));
                }
            }
        }
    }

    let summary = if error_count == 0 {
        strings
            .compressed_elements
            .replacen("{}", &success_count.to_string(), 1)
            .replacen("{}", &format_size(total_original), 1)
            .replacen("{}", &format_size(total_compressed), 1)
            .replacen("{}", &format_ratio(total_original, total_compressed), 1)
            .prepend("✅ ")
    } else {
        format!(
            "⚠️ {}",
            strings
                .compressed_with_errors
                .replacen("{}", &success_count.to_string(), 1)
                .replacen("{}", &error_count.to_string(), 1)
        )
    };

    TaskResult {
        success: error_count == 0,
        message: summary,
        details,
    }
}

/// Decomprime i file selezionati
fn decompress_files(
    files: &[PathBuf],
    force: bool,
    output_dir: Option<&Path>,
    lang: Language,
    ctx: &TaskContext,
) -> TaskResult {
    let strings = get_strings(lang);
    let mut success_count = 0;
    let mut error_count = 0;
    let mut details = Vec::new();

    let mut options = DecompressOptions::new().with_force(force);

    if let Some(dir) = output_dir {
        options = options.with_output_path(dir);
    }

    let total_files = files.len();
    for (idx, file) in files.iter().enumerate() {
        // Controlla flag cancellazione
        if ctx.cancel_flag.load(Ordering::Relaxed) != 0 {
            details.push("❌ Operazione annullata dall'utente".to_string());
            break;
        }

        // Invia progress
        let progress = (idx as f32) / (total_files as f32);
        let _ = ctx.progress_tx.send(TaskMessage::Progress(progress));
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "zst" {
            error_count += 1;
            details.push(format!(
                "⏭️ {:?}: {}",
                file.file_name().unwrap_or_default(),
                strings.not_zst_file
            ));
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
                details.push(format!(
                    "❌ {:?}: {}",
                    file.file_name().unwrap_or_default(),
                    e
                ));
            }
        }
    }

    let summary = if error_count == 0 {
        format!(
            "✅ {}",
            strings
                .decompressed_success
                .replacen("{}", &success_count.to_string(), 1)
        )
    } else {
        format!(
            "⚠️ {}",
            strings
                .decompressed_with_errors
                .replacen("{}", &success_count.to_string(), 1)
                .replacen("{}", &error_count.to_string(), 1)
        )
    };

    TaskResult {
        success: error_count == 0,
        message: summary,
        details,
    }
}

/// Verifica l'integrità dei file
fn verify_files(files: &[PathBuf], lang: Language, ctx: &TaskContext) -> TaskResult {
    let strings = get_strings(lang);
    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut skipped_count = 0;
    let mut details = Vec::new();

    let total_files = files.len();
    for (idx, file) in files.iter().enumerate() {
        // Controlla flag cancellazione
        if ctx.cancel_flag.load(Ordering::Relaxed) != 0 {
            details.push("❌ Operazione annullata dall'utente".to_string());
            break;
        }

        // Invia progress
        let progress = (idx as f32) / (total_files as f32);
        let _ = ctx.progress_tx.send(TaskMessage::Progress(progress));
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "zst" {
            skipped_count += 1;
            details.push(format!(
                "⏭️ {:?}: {}",
                file.file_name().unwrap_or_default(),
                strings.not_zst_file
            ));
            continue;
        }

        match verify_zst(file, None) {
            Ok(result) => {
                valid_count += 1;
                details.push(format!(
                    "✅ {:?}: {} ({} -> {})",
                    file.file_name().unwrap_or_default(),
                    strings.valid,
                    format_size(result.compressed_size),
                    format_size(result.decompressed_size)
                ));
            }
            Err(e) => {
                invalid_count += 1;
                details.push(format!(
                    "❌ {:?}: {}",
                    file.file_name().unwrap_or_default(),
                    e
                ));
            }
        }
    }

    let summary = if invalid_count == 0 && skipped_count == 0 {
        format!(
            "✅ {}",
            strings
                .files_valid
                .replacen("{}", &valid_count.to_string(), 1)
        )
    } else if invalid_count == 0 {
        format!(
            "✅ {}",
            strings
                .files_valid_skipped
                .replacen("{}", &valid_count.to_string(), 1)
                .replacen("{}", &skipped_count.to_string(), 1)
        )
    } else {
        format!(
            "⚠️ {}",
            strings
                .files_valid_corrupt_skipped
                .replacen("{}", &valid_count.to_string(), 1)
                .replacen("{}", &invalid_count.to_string(), 1)
                .replacen("{}", &skipped_count.to_string(), 1)
        )
    };

    TaskResult {
        success: invalid_count == 0,
        message: summary,
        details,
    }
}

trait PrependStr {
    fn prepend(self, s: &str) -> String;
}

impl PrependStr for String {
    fn prepend(self, s: &str) -> String {
        format!("{}{}", s, self)
    }
}
