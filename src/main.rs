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
    let input_size = std::fs::metadata(input_path)?.len() as f64 / 1_048_576.0;
    let output_size = std::fs::metadata(&output_path)?.len() as f64 / 1_048_576.0;
    println!("Dimensione originale: {:.2} MB -> Dimensione compressa: {:.2} MB", input_size, output_size);

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
    let input_size = std::fs::metadata(input_path)?.len() as f64 / 1_048_576.0;
    let output_size = std::fs::metadata(&output_path)?.len() as f64 / 1_048_576.0;
    println!("Dimensione compressa: {:.2} MB -> Dimensione originale: {:.2} MB", input_size, output_size);

    Ok(())
}
