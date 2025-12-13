use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::process;

/// Un programma per comprimere e decomprimere file con l'algoritmo Zstandard
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Comprime un file
    Compress {
        /// Il file da comprimere
        #[arg(value_name = "FILE")]
        input_file: PathBuf,

        /// Livello di compressione (da 1 a 21)
        #[arg(short, long, default_value_t = 3, value_parser = parse_level, value_name = "LIVELLO")]
        livello: i32,

        /// Sovrascrive il file di output se esiste già
        #[arg(short, long)]
        force: bool,
    },
    /// Decomprime un file con estensione .zst
    Decompress {
        /// Il file .zst da decomprimere
        #[arg(value_name = "FILE")]
        input_file: PathBuf,

        /// Sovrascrive il file di output se esiste già
        #[arg(short, long)]
        force: bool,
    },
}

/// Formatta una dimensione in bytes in modo leggibile (KB o MB)
fn format_size(bytes: u64) -> String {
    if bytes < 1_048_576 {
        // Meno di 1 MB, mostra in KB
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        // 1 MB o più, mostra in MB
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    }
}

/// Calcola e formatta il ratio di compressione
fn format_ratio(original: u64, compressed: u64) -> String {
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

fn parse_level(s: &str) -> Result<i32, String> {
    let v: i32 = s.parse().map_err(|_| {
        format!("Valore '{}' non valido: specifica un intero tra 1 e 21", s)
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

fn main() {
    // Analizza gli argomenti della riga di comando
    let cli = Cli::parse();

    // Esegue l'azione in base al sottocomando fornito (compress o decompress)
    let result = match &cli.command {
        Commands::Compress { input_file, livello, force } => {
            compress_file(input_file.as_path(), *livello, *force)
        }
        Commands::Decompress { input_file, force } => {
            decompress_file(input_file.as_path(), *force)
        }
    };

    // Gestisce gli errori con exit code appropriato
    if let Err(e) = result {
        eprintln!("Errore: {}", e);
        process::exit(1);
    }
}

/// Funzione per comprimere un file
fn compress_file(input_path: &Path, level: i32, force: bool) -> std::io::Result<()> {
    // Verifica che il file di input esista
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path)
        ));
    }

    // Costruisci il percorso di output:
    // se il file ha estensione, aggiunge ".zst" all'estensione corrente (es. file.txt -> file.txt.zst)
    // altrimenti imposta estensione "zst" (es. file -> file.zst)
    let output_path = match input_path.extension() {
        Some(ext) => {
            let mut new_ext: OsString = ext.to_os_string();
            new_ext.push(".zst");
            input_path.with_extension(new_ext)
        }
        None => input_path.with_extension("zst"),
    };
    if output_path.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    println!("File di input: {:?}", input_path);
    println!("File di output: {:?}", output_path);
    println!("Livello di compressione: {}", level);

    // Usa streaming compression per file grandi
    let input_file = File::open(input_path)?;
    let output_file = File::create(&output_path)?;
    
    let mut reader = BufReader::with_capacity(64 * 1024, input_file); // 64KB buffer
    let mut writer = BufWriter::with_capacity(64 * 1024, output_file); // 64KB buffer
    
    // Usa streaming encoder per evitare di caricare tutto in memoria
    let mut encoder = zstd::Encoder::new(&mut writer, level)?;
    std::io::copy(&mut reader, &mut encoder)?;
    // encoder.finish() chiude l'encoder e fa automaticamente il flush del writer
    encoder.finish()?;

    println!("\n✅ Compressione completata con successo!");

    // Calcola le dimensioni dei file per statistiche
    let input_size = std::fs::metadata(input_path)?.len();
    let output_size = std::fs::metadata(&output_path)?.len();
    println!(
        "Dimensione originale: {} -> Dimensione compressa: {} ({})",
        format_size(input_size),
        format_size(output_size),
        format_ratio(input_size, output_size)
    );

    Ok(())
}

/// Funzione per decomprimere un file
fn decompress_file(input_path: &Path, force: bool) -> std::io::Result<()> {
    // Verifica che il file di input esista
    if !input_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Il file di input {:?} non esiste", input_path)
        ));
    }

    // Controlla che il file abbia l'estensione .zst
    if input_path.extension().and_then(std::ffi::OsStr::to_str) != Some("zst") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Il file di input deve avere estensione .zst"));
    }

    // Crea il nome del file di output rimuovendo l'estensione .zst
    // Gestisce correttamente sia "file.txt.zst" -> "file.txt" che "file.zst" -> "file"
    let output_path = input_path.with_extension("");
    println!("File di input: {:?}", input_path);
    println!("File di output: {:?}", output_path);

    if output_path.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Il file di output {:?} esiste già. Usa --force per sovrascrivere.",
                output_path
            ),
        ));
    }

    // Usa streaming decompression per file grandi
    let input_file = File::open(input_path)?;
    let output_file = File::create(&output_path)?;
    
    let mut reader = BufReader::with_capacity(64 * 1024, input_file); // 64KB buffer
    let mut writer = BufWriter::with_capacity(64 * 1024, output_file); // 64KB buffer
    
    // Usa streaming decoder per evitare di caricare tutto in memoria
    let mut decoder = zstd::Decoder::new(&mut reader)?;
    std::io::copy(&mut decoder, &mut writer)?;

    // Forza il flush del buffer
    writer.flush()?;

    println!("\n✅ Decompressione completata con successo!");

    // Calcola le dimensioni dei file per statistiche
    let input_size = std::fs::metadata(input_path)?.len();
    let output_size = std::fs::metadata(&output_path)?.len();
    println!(
        "Dimensione compressa: {} -> Dimensione originale: {} ({})",
        format_size(input_size),
        format_size(output_size),
        format_ratio(output_size, input_size)
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Crea un file temporaneo con contenuto specifico per i test
    fn create_temp_file(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    /// Rimuove i file temporanei creati durante i test
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
        assert_eq!(format_size(1_572_864), "1.50 MB"); // 1.5 MB
    }

    #[test]
    fn test_format_ratio_reduction() {
        // 50% riduzione (da 100 a 50)
        assert_eq!(format_ratio(100, 50), "50.0% riduzione");
        // 75% riduzione (da 100 a 25)
        assert_eq!(format_ratio(100, 25), "75.0% riduzione");
    }

    #[test]
    fn test_format_ratio_increase() {
        // File che aumenta di dimensione (da 100 a 150)
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
        let original_content = b"Questo e' un testo di prova per testare la compressione e decompressione.\n".repeat(100);
        let input_path = create_temp_file("test_roundtrip.txt", &original_content);
        let compressed_path = input_path.with_extension("txt.zst");
        let decompressed_path = input_path.clone();

        // Comprime
        compress_file(&input_path, 3, true).unwrap();
        assert!(compressed_path.exists());

        // Rimuove il file originale per la decompressione
        fs::remove_file(&input_path).unwrap();

        // Decomprime
        decompress_file(&compressed_path, true).unwrap();
        assert!(decompressed_path.exists());

        // Verifica che il contenuto sia identico
        let decompressed_content = fs::read(&decompressed_path).unwrap();
        assert_eq!(original_content, decompressed_content.as_slice());

        // Cleanup
        cleanup_files(&[&input_path, &compressed_path]);
    }

    #[test]
    fn test_compress_file_not_found() {
        let result = compress_file(Path::new("/tmp/file_che_non_esiste_12345.txt"), 3, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_decompress_file_not_found() {
        let result = decompress_file(Path::new("/tmp/file_che_non_esiste_12345.zst"), false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_decompress_wrong_extension() {
        let input_path = create_temp_file("test_wrong_ext.txt", b"test content");

        let result = decompress_file(&input_path, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

        cleanup_files(&[&input_path]);
    }

    #[test]
    fn test_compress_no_force_existing_file() {
        let input_path = create_temp_file("test_no_force.txt", b"test content");
        let output_path = input_path.with_extension("txt.zst");

        // Crea il file di output
        File::create(&output_path).unwrap();

        // Prova a comprimere senza --force
        let result = compress_file(&input_path, 3, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);

        cleanup_files(&[&input_path, &output_path]);
    }

    #[test]
    fn test_compress_with_force_existing_file() {
        let input_path = create_temp_file("test_with_force.txt", b"test content for force test");
        let output_path = input_path.with_extension("txt.zst");

        // Crea il file di output
        File::create(&output_path).unwrap();

        // Comprime con --force
        let result = compress_file(&input_path, 3, true);
        assert!(result.is_ok());

        cleanup_files(&[&input_path, &output_path]);
    }

    #[test]
    fn test_compression_levels() {
        let content = b"Test content for compression level testing.\n".repeat(50);

        // Testa livello basso (veloce, meno compressione)
        let input_low = create_temp_file("test_level_low.txt", &content);
        compress_file(&input_low, 1, true).unwrap();
        let size_low = fs::metadata(input_low.with_extension("txt.zst")).unwrap().len();

        // Testa livello alto (lento, più compressione)
        let input_high = create_temp_file("test_level_high.txt", &content);
        compress_file(&input_high, 19, true).unwrap();
        let size_high = fs::metadata(input_high.with_extension("txt.zst")).unwrap().len();

        // Il livello alto dovrebbe produrre file più piccoli o uguali
        assert!(size_high <= size_low);

        cleanup_files(&[
            &input_low,
            &input_low.with_extension("txt.zst"),
            &input_high,
            &input_high.with_extension("txt.zst"),
        ]);
    }
}
