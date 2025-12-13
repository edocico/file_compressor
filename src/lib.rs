//! Libreria condivisa per la compressione/decompressione di file con Zstandard.
//!
//! Fornisce funzioni per comprimere e decomprimere file singoli, directory,
//! e archivi tar.zst.

use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

/// Dimensione del buffer per operazioni I/O (256KB per migliori performance)
pub const BUFFER_SIZE: usize = 256 * 1024;

/// Formatta una dimensione in bytes in modo leggibile (KB, MB, GB, TB)
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    }
}

/// Calcola e formatta il ratio di compressione
pub fn format_ratio(original: u64, compressed: u64) -> String {
    if original == 0 {
        return "N/A".to_string();
    }
    let ratio = (1.0 - (compressed as f64 / original as f64)) * 100.0;
    if ratio >= 0.0 {
        format!("{:.1}% riduzione", ratio)
    } else {
        format!("{:.1}% aumento", -ratio)
    }
}

/// Valida il livello di compressione (1-21)
pub fn parse_level(s: &str) -> Result<i32, String> {
    let v: i32 = s.parse().map_err(|_| {
        format!(
            "Valore '{}' non valido: specifica un intero tra 1 e 21",
            s
        )
    })?;
    if (1..=21).contains(&v) {
        Ok(v)
    } else {
        Err(format!(
            "Livello {} fuori intervallo: usa un valore tra 1 e 21",
            v
        ))
    }
}

/// Ritorna il numero di CPU disponibili
pub fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

/// Costruisce il path di output per la compressione
pub fn build_output_path(input_path: &Path) -> PathBuf {
    match input_path.extension() {
        Some(ext) => {
            let mut new_ext: OsString = ext.to_os_string();
            new_ext.push(".zst");
            input_path.with_extension(new_ext)
        }
        None => input_path.with_extension("zst"),
    }
}

/// Risultato di un'operazione di compressione/decompressione
#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub input_size: u64,
    pub output_size: u64,
}

/// Callback per aggiornare il progresso
pub type ProgressCallback = Box<dyn Fn(u64) + Send + Sync>;

/// Opzioni per la compressione
#[derive(Default)]
pub struct CompressOptions {
    pub level: i32,
    pub force: bool,
    pub parallel: bool,
    pub progress_callback: Option<ProgressCallback>,
}

impl CompressOptions {
    pub fn new(level: i32) -> Self {
        Self {
            level,
            force: false,
            parallel: false,
            progress_callback: None,
        }
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }
}

/// Comprime un singolo file
pub fn compress_file(input_path: &Path, options: &CompressOptions) -> std::io::Result<CompressionResult> {
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path),
        ));
    }

    let output_path = build_output_path(input_path);

    if output_path.exists() && !options.force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    let input_size = std::fs::metadata(input_path)?.len();
    let input_file = File::open(input_path)?;
    let output_file = File::create(&output_path)?;

    let mut reader = BufReader::with_capacity(BUFFER_SIZE, input_file);
    let writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);

    let mut encoder = zstd::Encoder::new(writer, options.level)?;

    // Abilita multithreading se richiesto
    if options.parallel {
        encoder.set_parameter(zstd::zstd_safe::CParameter::NbWorkers(num_cpus()))?;
    }

    // Buffer per la lettura incrementale con progress
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_read = 0u64;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&buffer[..bytes_read])?;
        total_read += bytes_read as u64;

        if let Some(ref callback) = options.progress_callback {
            callback(total_read);
        }
    }

    encoder.finish()?;

    let output_size = std::fs::metadata(&output_path)?.len();

    Ok(CompressionResult {
        input_size,
        output_size,
    })
}

/// Comprime un singolo file (versione semplice senza progress)
pub fn compress_file_simple(input_path: &Path, level: i32, force: bool) -> std::io::Result<()> {
    let options = CompressOptions::new(level).with_force(force);
    compress_file(input_path, &options)?;
    Ok(())
}

/// Comprime una directory in un archivio tar.zst
pub fn compress_directory(dir_path: &Path, options: &CompressOptions) -> std::io::Result<CompressionResult> {
    if !dir_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("La directory {:?} non esiste", dir_path),
        ));
    }

    if !dir_path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{:?} non è una directory", dir_path),
        ));
    }

    let dir_name = dir_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("archivio"));
    let output_path = dir_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{}.tar.zst", dir_name.to_string_lossy()));

    if output_path.exists() && !options.force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    // Calcola la dimensione totale della directory
    let total_size = calculate_dir_size(dir_path)?;

    let output_file = File::create(&output_path)?;
    let writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);
    let mut encoder = zstd::Encoder::new(writer, options.level)?;

    if options.parallel {
        encoder.set_parameter(zstd::zstd_safe::CParameter::NbWorkers(num_cpus()))?;
    }

    let mut tar = Builder::new(encoder);

    // Aggiungi tutti i file dalla directory con progress tracking
    let progress_tracker = ProgressTracker::new(options.progress_callback.as_ref());
    add_dir_to_tar_with_progress(&mut tar, dir_path, dir_path, &progress_tracker)?;

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    let output_size = std::fs::metadata(&output_path)?.len();

    Ok(CompressionResult {
        input_size: total_size,
        output_size,
    })
}

/// Calcola la dimensione totale di una directory
pub fn calculate_dir_size(dir: &Path) -> std::io::Result<u64> {
    let mut size = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            size += calculate_dir_size(&path)?;
        } else {
            size += std::fs::metadata(&path)?.len();
        }
    }
    Ok(size)
}

/// Conta i file in una directory ricorsivamente
pub fn count_files_in_dir(dir: &Path) -> std::io::Result<u64> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_files_in_dir(&path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

/// Tracker per il progresso
struct ProgressTracker<'a> {
    callback: Option<&'a ProgressCallback>,
    processed: std::cell::Cell<u64>,
}

impl<'a> ProgressTracker<'a> {
    fn new(callback: Option<&'a ProgressCallback>) -> Self {
        Self {
            callback,
            processed: std::cell::Cell::new(0),
        }
    }

    fn add(&self, bytes: u64) {
        let new_total = self.processed.get() + bytes;
        self.processed.set(new_total);
        if let Some(callback) = self.callback {
            callback(new_total);
        }
    }
}

/// Aggiunge una directory al tar ricorsivamente con progress tracking
fn add_dir_to_tar_with_progress<W: Write>(
    tar: &mut Builder<W>,
    base_path: &Path,
    current_path: &Path,
    progress: &ProgressTracker,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(base_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        if path.is_dir() {
            add_dir_to_tar_with_progress(tar, base_path, &path, progress)?;
        } else {
            let file_size = std::fs::metadata(&path)?.len();
            tar.append_path_with_name(&path, relative_path)?;
            progress.add(file_size);
        }
    }
    Ok(())
}

/// Aggiunge una directory al tar ricorsivamente (versione semplice)
pub fn add_dir_to_tar<W: Write>(
    tar: &mut Builder<W>,
    base_path: &Path,
    current_path: &Path,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(base_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        if path.is_dir() {
            add_dir_to_tar(tar, base_path, &path)?;
        } else {
            tar.append_path_with_name(&path, relative_path)?;
        }
    }
    Ok(())
}

/// Comprime più file in un singolo archivio tar.zst
pub fn compress_multiple_files(
    input_files: &[PathBuf],
    output_path: &Path,
    options: &CompressOptions,
) -> std::io::Result<CompressionResult> {
    // Verifica che tutti i file esistano
    for file in input_files {
        if !file.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Il file {:?} non esiste", file),
            ));
        }
    }

    if output_path.exists() && !options.force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    let output_file = File::create(output_path)?;
    let writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);
    let mut encoder = zstd::Encoder::new(writer, options.level)?;

    if options.parallel {
        encoder.set_parameter(zstd::zstd_safe::CParameter::NbWorkers(num_cpus()))?;
    }

    let mut tar = Builder::new(encoder);
    let mut total_input_size = 0u64;
    let mut processed = 0u64;

    for file in input_files {
        let file_name = file.file_name().unwrap_or_else(|| std::ffi::OsStr::new("file"));
        let file_size = std::fs::metadata(file)?.len();
        tar.append_path_with_name(file, file_name)?;
        total_input_size += file_size;
        processed += file_size;

        if let Some(ref callback) = options.progress_callback {
            callback(processed);
        }
    }

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    let output_size = std::fs::metadata(output_path)?.len();

    Ok(CompressionResult {
        input_size: total_input_size,
        output_size,
    })
}

/// Opzioni per la decompressione
#[derive(Default)]
pub struct DecompressOptions {
    pub force: bool,
    pub progress_callback: Option<ProgressCallback>,
}

impl DecompressOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }
}

/// Decomprime un file .zst o .tar.zst
pub fn decompress_file(input_path: &Path, options: &DecompressOptions) -> std::io::Result<CompressionResult> {
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path),
        ));
    }

    let extension = input_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    if extension != "zst" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Il file di input deve avere estensione .zst",
        ));
    }

    let is_tar = input_path.to_string_lossy().ends_with(".tar.zst");

    if is_tar {
        decompress_tar_zst(input_path, options)
    } else {
        decompress_single_file(input_path, options)
    }
}

/// Decomprime un singolo file .zst
pub fn decompress_single_file(input_path: &Path, options: &DecompressOptions) -> std::io::Result<CompressionResult> {
    let output_path = input_path.with_extension("");

    if output_path.exists() && !options.force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    let input_size = std::fs::metadata(input_path)?.len();
    let input_file = File::open(input_path)?;
    let output_file = File::create(&output_path)?;

    let reader = BufReader::with_capacity(BUFFER_SIZE, input_file);
    let mut writer = BufWriter::with_capacity(BUFFER_SIZE, output_file);

    let mut decoder = zstd::Decoder::new(reader)?;

    let mut buffer = vec![0u8; BUFFER_SIZE];
    #[allow(unused_assignments)]
    let mut total_written = 0u64;
    let mut last_progress_update = 0u64;

    loop {
        let bytes_read = decoder.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        total_written += bytes_read as u64;

        // Aggiorna progress ogni 1MB per evitare troppe chiamate
        if let Some(ref callback) = options.progress_callback {
            if total_written - last_progress_update >= 1024 * 1024 {
                // Stima del progresso basata sul ratio tipico di compressione
                let estimated_progress = (total_written as f64 / 3.0).min(input_size as f64) as u64;
                callback(estimated_progress);
                last_progress_update = total_written;
            }
        }
    }

    writer.flush()?;

    // Notifica completamento
    if let Some(ref callback) = options.progress_callback {
        callback(input_size);
    }

    let output_size = std::fs::metadata(&output_path)?.len();

    Ok(CompressionResult {
        input_size,
        output_size,
    })
}

/// Decomprime un archivio tar.zst
pub fn decompress_tar_zst(input_path: &Path, options: &DecompressOptions) -> std::io::Result<CompressionResult> {
    let file_stem = input_path
        .file_stem()
        .and_then(|s| Path::new(s).file_stem())
        .unwrap_or_else(|| std::ffi::OsStr::new("output"));

    let output_dir = input_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(file_stem);

    if output_dir.exists() && !options.force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "La directory di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_dir
            ),
        ));
    }

    std::fs::create_dir_all(&output_dir)?;

    let input_size = std::fs::metadata(input_path)?.len();
    let input_file = File::open(input_path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, input_file);
    let decoder = zstd::Decoder::new(reader)?;
    let mut archive = Archive::new(decoder);

    let mut file_count = 0u64;
    let mut total_extracted = 0u64;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let dest_path = output_dir.join(&path);

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let entry_size = entry.size();
        entry.unpack(&dest_path)?;
        file_count += 1;
        total_extracted += entry_size;

        if let Some(ref callback) = options.progress_callback {
            callback(file_count);
        }
    }

    Ok(CompressionResult {
        input_size,
        output_size: total_extracted,
    })
}

/// Decomprime un file (versione semplice)
pub fn decompress_file_simple(input_path: &Path, force: bool) -> std::io::Result<()> {
    let options = DecompressOptions::new().with_force(force);
    decompress_file(input_path, &options)?;
    Ok(())
}

/// Verifica l'integrità di un file .zst
pub fn verify_zst(input_path: &Path, progress_callback: Option<&ProgressCallback>) -> std::io::Result<VerifyResult> {
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file {:?} non esiste", input_path),
        ));
    }

    let extension = input_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");

    if extension != "zst" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Il file deve avere estensione .zst",
        ));
    }

    let input_size = std::fs::metadata(input_path)?.len();
    let input_file = File::open(input_path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, input_file);

    let mut decoder = match zstd::Decoder::new(reader) {
        Ok(d) => d,
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("File corrotto: impossibile inizializzare il decoder: {}", e),
            ));
        }
    };

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_decompressed = 0u64;
    let mut last_progress_update = 0u64;

    loop {
        match decoder.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                total_decompressed += n as u64;

                if let Some(callback) = progress_callback {
                    if total_decompressed - last_progress_update >= 1024 * 1024 {
                        let progress = (total_decompressed as f64 / 3.0).min(input_size as f64) as u64;
                        callback(progress);
                        last_progress_update = total_decompressed;
                    }
                }
            }
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("File corrotto: errore durante la decompressione: {}", e),
                ));
            }
        }
    }

    // Notifica completamento
    if let Some(callback) = progress_callback {
        callback(input_size);
    }

    Ok(VerifyResult {
        compressed_size: input_size,
        decompressed_size: total_decompressed,
    })
}

/// Risultato della verifica
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub compressed_size: u64,
    pub decompressed_size: u64,
}

/// Verifica l'integrità di un file .zst (versione semplice)
pub fn verify_zst_simple(input_path: &Path) -> std::io::Result<()> {
    verify_zst(input_path, None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_file(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    fn cleanup_files(paths: &[&Path]) {
        for path in paths {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(512), "0.50 KB");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(512 * 1024), "512.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(5 * 1_048_576), "5.00 MB");
        assert_eq!(format_size(1_572_864), "1.50 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
        assert_eq!(format_size(5 * 1_073_741_824), "5.00 GB");
    }

    #[test]
    fn test_format_size_tb() {
        assert_eq!(format_size(1_099_511_627_776), "1.00 TB");
    }

    #[test]
    fn test_format_ratio_reduction() {
        assert_eq!(format_ratio(100, 50), "50.0% riduzione");
        assert_eq!(format_ratio(100, 25), "75.0% riduzione");
    }

    #[test]
    fn test_format_ratio_increase() {
        assert_eq!(format_ratio(100, 150), "50.0% aumento");
    }

    #[test]
    fn test_format_ratio_zero() {
        assert_eq!(format_ratio(0, 100), "N/A");
    }

    #[test]
    fn test_parse_level_valid() {
        assert_eq!(parse_level("1").unwrap(), 1);
        assert_eq!(parse_level("10").unwrap(), 10);
        assert_eq!(parse_level("21").unwrap(), 21);
    }

    #[test]
    fn test_parse_level_invalid_range() {
        assert!(parse_level("0").is_err());
        assert!(parse_level("22").is_err());
        assert!(parse_level("-1").is_err());
    }

    #[test]
    fn test_parse_level_invalid_format() {
        assert!(parse_level("abc").is_err());
        assert!(parse_level("").is_err());
        assert!(parse_level("3.5").is_err());
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original_content =
            b"Questo e' un testo di prova per testare la compressione e decompressione.\n"
                .repeat(100);
        let input_path = create_temp_file("test_lib_roundtrip.txt", &original_content);
        let compressed_path = input_path.with_extension("txt.zst");
        let decompressed_path = input_path.clone();

        // Comprime
        compress_file_simple(&input_path, 3, true).unwrap();
        assert!(compressed_path.exists());

        // Rimuove il file originale per la decompressione
        fs::remove_file(&input_path).unwrap();

        // Decomprime
        decompress_file_simple(&compressed_path, true).unwrap();
        assert!(decompressed_path.exists());

        // Verifica che il contenuto sia identico
        let decompressed_content = fs::read(&decompressed_path).unwrap();
        assert_eq!(original_content, decompressed_content.as_slice());

        // Cleanup
        cleanup_files(&[&input_path, &compressed_path]);
    }

    #[test]
    fn test_verify_valid_zst() {
        let original_content = b"Test content for verification.\n".repeat(50);
        let input_path = create_temp_file("test_lib_verify.txt", &original_content);
        let compressed_path = input_path.with_extension("txt.zst");

        compress_file_simple(&input_path, 3, true).unwrap();

        let result = verify_zst_simple(&compressed_path);
        assert!(result.is_ok());

        cleanup_files(&[&input_path, &compressed_path]);
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus >= 1);
    }
}
