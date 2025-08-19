use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

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
        #[arg(short, long, default_value_t = 3)]
        livello: i32,
    },
    /// Decomprime un file con estensione .zst
    Decompress {
        /// Il file .zst da decomprimere
        #[arg(value_name = "FILE")]
        input_file: PathBuf,
    },
}

fn main() {
    // Analizza gli argomenti della riga di comando
    let cli = Cli::parse();

    // Esegue l'azione in base al sottocomando fornito (compress o decompress)
    match &cli.command {
        Commands::Compress { input_file, livello } => {
            if let Err(e) = compress_file(input_file, *livello) {
                eprintln!("Errore durante la compressione: {}", e);
            }
        }
        Commands::Decompress { input_file } => {
            if let Err(e) = decompress_file(input_file) {
                eprintln!("Errore durante la decompressione: {}", e);
            }
        }
    }
}

/// Funzione per comprimere un file
fn compress_file(input_path: &PathBuf, level: i32) -> std::io::Result<()> {
    // Controlla che il file esista
    if !input_path.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "File di input non trovato"));
    }
    
    // Crea il nome del file di output usando OsString per evitare allocazioni
    let mut output_path = input_path.as_os_str().to_owned();
    output_path.push(".zst");
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
    encoder.finish()?;
    
    // Forza il flush del buffer
    writer.flush()?;

    println!("\n✅ Compressione completata con successo!");
    
    // Calcola le dimensioni dei file per statistiche
    let input_size = std::fs::metadata(input_path)?.len() as f64 / 1_048_576.0;
    let output_size = std::fs::metadata(&output_path)?.len() as f64 / 1_048_576.0;
    println!("Dimensione originale: {:.2} MB -> Dimensione compressa: {:.2} MB", input_size, output_size);

    Ok(())
}

/// Funzione per decomprimere un file
fn decompress_file(input_path: &PathBuf) -> std::io::Result<()> {
    // Controlla che il file abbia l'estensione .zst
    if input_path.extension().and_then(std::ffi::OsStr::to_str) != Some("zst") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Il file di input deve avere estensione .zst"));
    }

    // Crea il nome del file di output rimuovendo l'estensione .zst
    let output_path = input_path.with_extension("");
    println!("File di input: {:?}", input_path);
    println!("File di output: {:?}", output_path);

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

    Ok(())
}
