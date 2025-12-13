use clap::{Parser, Subcommand};
use file_compressor::{
    compress_directory, compress_file, compress_file_simple, compress_multiple_files,
    count_files_in_dir, decompress_single_file, decompress_tar_zst, format_ratio, format_size,
    parse_level, verify_zst, CompressOptions, DecompressOptions, ProgressCallback,
};
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Un programma per comprimere e decomprimere file con l'algoritmo Zstandard
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Comprime un file o una directory
    Compress {
        /// Il file o la directory da comprimere
        #[arg(value_name = "FILE")]
        input_file: PathBuf,

        /// Livello di compressione (da 1 a 21)
        #[arg(short, long, default_value_t = 3, value_parser = parse_level, value_name = "LIVELLO")]
        livello: i32,

        /// Sovrascrive il file di output se esiste già
        #[arg(short, long)]
        force: bool,

        /// Usa compressione multi-threaded per file grandi
        #[arg(short, long)]
        parallel: bool,

        /// Percorso di destinazione (file o directory)
        #[arg(short, long, value_name = "PERCORSO")]
        output: Option<PathBuf>,
    },
    /// Decomprime un file con estensione .zst o .tar.zst
    Decompress {
        /// Il file .zst da decomprimere
        #[arg(value_name = "FILE")]
        input_file: PathBuf,

        /// Sovrascrive il file di output se esiste già
        #[arg(short, long)]
        force: bool,

        /// Percorso di destinazione (file o directory)
        #[arg(short, long, value_name = "PERCORSO")]
        output: Option<PathBuf>,
    },
    /// Comprime più file in un archivio tar.zst
    MultiCompress {
        /// I file da comprimere
        #[arg(value_name = "FILES", num_args = 1..)]
        input_files: Vec<PathBuf>,

        /// Nome del file di output (default: archivio.tar.zst)
        #[arg(short, long, default_value = "archivio.tar.zst")]
        output: PathBuf,

        /// Livello di compressione (da 1 a 21)
        #[arg(short, long, default_value_t = 3, value_parser = parse_level, value_name = "LIVELLO")]
        livello: i32,

        /// Sovrascrive il file di output se esiste già
        #[arg(short, long)]
        force: bool,
    },
    /// Comprime tutti i file che corrispondono a un pattern (es. *.log)
    Batch {
        /// Il pattern glob da cercare (es. "*.log", "**/*.txt")
        #[arg(value_name = "PATTERN")]
        pattern: String,

        /// Livello di compressione (da 1 a 21)
        #[arg(short, long, default_value_t = 3, value_parser = parse_level, value_name = "LIVELLO")]
        livello: i32,

        /// Sovrascrive i file di output se esistono già
        #[arg(short, long)]
        force: bool,

        /// Elabora i file in parallelo
        #[arg(short, long)]
        parallel: bool,
    },
    /// Verifica l'integrità di un file .zst
    Verifica {
        /// Il file .zst da verificare
        #[arg(value_name = "FILE")]
        input_file: PathBuf,
    },
}

/// Crea una progress bar con stile personalizzato
fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Crea una spinner per operazioni senza dimensione nota
fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb
}

/// Crea una progress bar per conteggio file
fn create_file_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} file")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Compress {
            input_file,
            livello,
            force,
            parallel,
            output,
        } => {
            if input_file.is_dir() {
                compress_directory_with_progress(input_file.as_path(), *livello, *force, output.as_deref())
            } else {
                compress_file_with_progress(input_file.as_path(), *livello, *force, *parallel, output.as_deref())
            }
        }
        Commands::Decompress { input_file, force, output } => {
            decompress_file_with_progress(input_file.as_path(), *force, output.as_deref())
        }
        Commands::MultiCompress {
            input_files,
            output,
            livello,
            force,
        } => compress_multiple_with_progress(input_files, output.as_path(), *livello, *force),
        Commands::Batch {
            pattern,
            livello,
            force,
            parallel,
        } => batch_compress(pattern, *livello, *force, *parallel),
        Commands::Verifica { input_file } => verify_with_progress(input_file.as_path()),
    };

    if let Err(e) = result {
        eprintln!("Errore: {}", e);
        process::exit(1);
    }
}

/// Comprime un file con progress bar
fn compress_file_with_progress(
    input_path: &Path,
    level: i32,
    force: bool,
    parallel: bool,
    output: Option<&Path>,
) -> std::io::Result<()> {
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path),
        ));
    }

    println!("File di input: {:?}", input_path);
    if let Some(out) = output {
        println!("Destinazione: {:?}", out);
    }
    println!(
        "Livello di compressione: {}{}",
        level,
        if parallel { " (modalità parallela)" } else { "" }
    );

    let input_size = std::fs::metadata(input_path)?.len();
    let pb = create_progress_bar(input_size, "Compressione in corso...");
    let pb_clone = pb.clone();

    let mut options = CompressOptions::new(level)
        .with_force(force)
        .with_parallel(parallel)
        .with_progress(move |bytes| {
            pb_clone.set_position(bytes);
        });

    if let Some(out) = output {
        options = options.with_output_path(out);
    }

    let result = compress_file(input_path, &options)?;

    pb.finish_with_message("Compressione completata!");

    println!("\n✅ Compressione completata con successo!");
    println!(
        "Dimensione originale: {} -> Dimensione compressa: {} ({})",
        format_size(result.input_size),
        format_size(result.output_size),
        format_ratio(result.input_size, result.output_size)
    );

    Ok(())
}

/// Comprime una directory con progress bar
fn compress_directory_with_progress(
    dir_path: &Path,
    level: i32,
    force: bool,
    output: Option<&Path>,
) -> std::io::Result<()> {
    println!("Directory di input: {:?}", dir_path);
    if let Some(out) = output {
        println!("Destinazione: {:?}", out);
    }
    println!("Livello di compressione: {}", level);

    let spinner = create_spinner("Analisi directory...");
    let file_count = count_files_in_dir(dir_path)?;
    spinner.finish_and_clear();

    let pb = create_file_progress_bar(file_count, "Compressione directory...");
    let pb_clone = pb.clone();
    let processed_files = Arc::new(AtomicU64::new(0));
    let processed_clone = Arc::clone(&processed_files);

    let mut options = CompressOptions::new(level)
        .with_force(force)
        .with_progress(move |_bytes| {
            // Incrementa il conteggio dei file
            let files = processed_clone.fetch_add(1, Ordering::Relaxed);
            pb_clone.set_position(files + 1);
        });

    if let Some(out) = output {
        options = options.with_output_path(out);
    }

    let result = compress_directory(dir_path, &options)?;

    pb.finish_with_message("Archivio creato!");

    println!("\n✅ Compressione directory completata con successo!");
    println!(
        "File nell'archivio: {} - Dimensione archivio: {} ({})",
        file_count,
        format_size(result.output_size),
        format_ratio(result.input_size, result.output_size)
    );

    Ok(())
}

/// Comprime più file con progress bar
fn compress_multiple_with_progress(
    input_files: &[PathBuf],
    output_path: &Path,
    level: i32,
    force: bool,
) -> std::io::Result<()> {
    println!("File da comprimere: {} file", input_files.len());
    println!("File di output: {:?}", output_path);
    println!("Livello di compressione: {}", level);

    let pb = create_file_progress_bar(input_files.len() as u64, "Compressione multi-file...");
    let pb_clone = pb.clone();
    let processed = Arc::new(AtomicU64::new(0));
    let processed_clone = Arc::clone(&processed);

    let options = CompressOptions::new(level)
        .with_force(force)
        .with_progress(move |_bytes| {
            let count = processed_clone.fetch_add(1, Ordering::Relaxed);
            pb_clone.set_position(count + 1);
        });

    let result = compress_multiple_files(input_files, output_path, &options)?;

    pb.finish_with_message("Archivio creato!");

    println!("\n✅ Compressione multi-file completata con successo!");
    println!(
        "Dimensione originale totale: {} -> Dimensione archivio: {} ({})",
        format_size(result.input_size),
        format_size(result.output_size),
        format_ratio(result.input_size, result.output_size)
    );

    Ok(())
}

/// Decomprime un file con progress bar
fn decompress_file_with_progress(input_path: &Path, force: bool, output: Option<&Path>) -> std::io::Result<()> {
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path),
        ));
    }

    let is_tar = input_path.to_string_lossy().ends_with(".tar.zst");

    println!("File di input: {:?}", input_path);
    if let Some(out) = output {
        println!("Destinazione: {:?}", out);
    }

    if is_tar {
        let spinner = create_spinner("Estrazione archivio tar.zst...");
        let spinner_clone = spinner.clone();
        let file_count = Arc::new(AtomicU64::new(0));
        let file_count_clone = Arc::clone(&file_count);

        let mut options = DecompressOptions::new()
            .with_force(force)
            .with_progress(move |files| {
                file_count_clone.store(files, Ordering::Relaxed);
                spinner_clone.set_message(format!("Estratti {} file...", files));
            });

        if let Some(out) = output {
            options = options.with_output_path(out);
        }

        let result = decompress_tar_zst(input_path, &options)?;
        let extracted = file_count.load(Ordering::Relaxed);
        spinner.finish_with_message(format!("Estrazione completata: {} file", extracted));

        println!("\n✅ Estrazione archivio completata con successo!");
        println!(
            "Dimensione archivio: {} - File estratti: {}",
            format_size(result.input_size),
            extracted
        );
    } else {
        let input_size = std::fs::metadata(input_path)?.len();
        let pb = create_progress_bar(input_size, "Decompressione in corso...");
        let pb_clone = pb.clone();

        let mut options = DecompressOptions::new()
            .with_force(force)
            .with_progress(move |bytes| {
                pb_clone.set_position(bytes);
            });

        if let Some(out) = output {
            options = options.with_output_path(out);
        }

        let result = decompress_single_file(input_path, &options)?;

        pb.finish_with_message("Decompressione completata!");

        println!("\n✅ Decompressione completata con successo!");
        println!(
            "Dimensione compressa: {} -> Dimensione originale: {} ({})",
            format_size(result.input_size),
            format_size(result.output_size),
            format_ratio(result.output_size, result.input_size)
        );
    }

    Ok(())
}

/// Comprime tutti i file che corrispondono a un pattern glob
fn batch_compress(pattern: &str, level: i32, force: bool, parallel: bool) -> std::io::Result<()> {
    let files: Vec<PathBuf> = glob(pattern)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
        .filter_map(|entry| entry.ok())
        .filter(|path| path.is_file())
        .collect();

    if files.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Nessun file trovato con il pattern '{}'", pattern),
        ));
    }

    println!("Trovati {} file con il pattern '{}'", files.len(), pattern);
    println!("Livello di compressione: {}", level);
    println!(
        "Modalità: {}",
        if parallel { "parallela" } else { "sequenziale" }
    );
    println!();

    let pb = create_file_progress_bar(files.len() as u64, "Compressione batch...");

    let success_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));

    if parallel {
        let pb_ref = &pb;
        let success_ref = &success_count;
        let error_ref = &error_count;

        files.par_iter().for_each(|file| {
            match compress_file_simple(file, level, force) {
                Ok(_) => {
                    success_ref.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    error_ref.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Errore comprimendo {:?}: {}", file, e);
                }
            }
            pb_ref.inc(1);
        });
    } else {
        for file in &files {
            match compress_file_simple(file, level, force) {
                Ok(_) => {
                    success_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    error_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Errore comprimendo {:?}: {}", file, e);
                }
            }
            pb.inc(1);
        }
    }

    pb.finish_with_message("Compressione batch completata!");

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("\n✅ Compressione batch completata!");
    println!("File compressi con successo: {}", successes);
    if errors > 0 {
        println!("⚠️  File con errori: {}", errors);
    }

    Ok(())
}

/// Verifica l'integrità di un file .zst con progress bar
fn verify_with_progress(input_path: &Path) -> std::io::Result<()> {
    println!("Verifica integrità: {:?}", input_path);

    let input_size = std::fs::metadata(input_path)?.len();
    let pb = create_progress_bar(input_size, "Verifica in corso...");
    let pb_clone = pb.clone();

    let callback: ProgressCallback = Box::new(move |bytes: u64| {
        pb_clone.set_position(bytes);
    });

    let result = match verify_zst(input_path, Some(&callback)) {
        Ok(r) => {
            pb.finish_with_message("Verifica completata!");
            r
        }
        Err(e) => {
            pb.finish_with_message("Verifica fallita!");
            return Err(e);
        }
    };

    println!("\n✅ Il file è integro e valido!");
    println!("Dimensione compressa: {}", format_size(result.compressed_size));
    println!(
        "Dimensione decompressa: {}",
        format_size(result.decompressed_size)
    );
    println!(
        "Ratio: {}",
        format_ratio(result.decompressed_size, result.compressed_size)
    );

    Ok(())
}
