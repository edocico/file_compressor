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

        // Pannello centrale
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🗜️ File Compressor");
            ui.add_space(10.0);

            // Selezione operazione
            ui.horizontal(|ui| {
                ui.label(strings.operation);
                ui.selectable_value(&mut self.operation, Operation::Compress, strings.compress);
                ui.selectable_value(
                    &mut self.operation,
                    Operation::Decompress,
                    strings.decompress,
                );
                ui.selectable_value(&mut self.operation, Operation::Verify, strings.verify);
            });

            ui.add_space(10.0);

            // Opzioni per compressione
            if matches!(self.operation, Operation::Compress) {
                ui.horizontal(|ui| {
                    ui.label(strings.compression_level);
                    ui.add(egui::Slider::new(&mut self.compression_level, 1..=21));
                    ui.label(compression_level_hint(
                        self.compression_level,
                        self.language,
                    ));
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.parallel, strings.parallel_compression);
                });
            }

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.force_overwrite, strings.overwrite_existing);
            });

            ui.add_space(5.0);

            // Selezione cartella di destinazione
            if !matches!(self.operation, Operation::Verify) {
                ui.horizontal(|ui| {
                    ui.label(strings.destination);
                    if let Some(ref dir) = self.output_directory {
                        let dir_name = dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| dir.to_string_lossy().to_string());
                        ui.label(format!("📁 {}", dir_name));
                        if ui.small_button("❌").clicked() {
                            self.output_directory = None;
                        }
                    } else {
                        ui.label(strings.same_folder);
                    }
                    if ui.button(format!("📂 {}", strings.choose)).clicked() && !self.is_processing
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_directory = Some(path);
                        }
                    }
                });
            }

            ui.add_space(10.0);

            // Area drag and drop
            let drop_area = ui.group(|ui| {
                ui.set_min_size(egui::vec2(ui.available_width(), 80.0));
                ui.centered_and_justified(|ui| {
                    if self.selected_files.is_empty() {
                        ui.label(format!("📁 {}", strings.drag_files_here));
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label(format!(
                                "📁 {} {}",
                                self.selected_files.len(),
                                strings.elements_selected
                            ));
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
                if ui.button(format!("📂 {}", strings.select_files)).clicked()
                    && !self.is_processing
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

                if ui.button(format!("📁 {}", strings.select_folder)).clicked()
                    && !self.is_processing
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

                if ui.button(format!("🗑️ {}", strings.clear)).clicked() && !self.is_processing
                {
                    self.selected_files.clear();
                    self.last_details.clear();
                    self.status_message = strings.drag_or_select.to_string();
                }
            });

            ui.add_space(10.0);

            // Pulsante esegui
            let button_text = match self.operation {
                Operation::Compress => format!("🗜️ {}", strings.compress_btn),
                Operation::Decompress => format!("📦 {}", strings.decompress_btn),
                Operation::Verify => format!("✅ {}", strings.verify_btn),
            };

            ui.add_enabled_ui(
                !self.is_processing && !self.selected_files.is_empty(),
                |ui| {
                    if ui.button(button_text).clicked() {
                        self.process_files();
                    }
                },
            );

            // Progress bar e pulsante annulla
            if self.is_processing {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                    if ui.button("⏹ Annulla").clicked() {
                        self.cancel_operation();
                    }
                });
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
                    let details_btn_text = if self.show_details {
                        format!("▼ {}", strings.hide_details)
                    } else {
                        format!("▶ {}", strings.show_details)
                    };
                    if ui.small_button(details_btn_text).clicked() {
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
