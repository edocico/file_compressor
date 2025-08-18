use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{Read, Write};
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
    
    // Crea il nome del file di output (es. 'mio_file.txt' -> 'mio_file.txt.zst')
    let output_path = input_path.as_os_str().to_str().unwrap().to_string() + ".zst";
    println!("File di input: {:?}", input_path);
    println!("File di output: {}", output_path);
    println!("Livello di compressione: {}", level);

    // Legge il file di input
    let mut input_file = File::open(input_path)?;
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;

    // Comprime i dati
    let compressed_data = zstd::encode_all(&buffer[..], level)?;

    // Scrive i dati compressi nel file di output
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&compressed_data)?;

    println!("\n✅ Compressione completata con successo!");
    let input_size = buffer.len() as f64 / 1_048_576.0; // in MB
    let output_size = compressed_data.len() as f64 / 1_048_576.0; // in MB
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

    // Legge il file compresso
    let mut input_file = File::open(input_path)?;
    let mut compressed_buffer = Vec::new();
    input_file.read_to_end(&mut compressed_buffer)?;

    // Decomprime i dati
    let decompressed_data = zstd::decode_all(&compressed_buffer[..])?;

    // Scrive i dati decompressi nel file di output
    let mut output_file = File::create(output_path)?;
    output_file.write_all(&decompressed_data)?;

    println!("\n✅ Decompressione completata con successo!");

    Ok(())
}
