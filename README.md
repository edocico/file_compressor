# File Compressor 🗜️

Un'utility di compressione/decompressione file ad alte prestazioni scritta in Rust utilizzando l'algoritmo **Zstandard (zstd)**. Fornisce sia interfaccia CLI che GUI con supporto per operazioni batch e compressione parallela.

[![Build Status](https://github.com/edocico/file_compressor/workflows/Build%20Release/badge.svg)](https://github.com/edocico/file_compressor/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

## ✨ Caratteristiche

- **🚀 Prestazioni Elevate**: Buffer adattivi, compressione multi-threaded automatica per file grandi
- **🎯 Flessibile**: Supporta file singoli, directory, archivi multi-file e batch processing
- **📊 Progress Tracking**: Barre di progresso dettagliate per tutte le operazioni
- **🌍 Multilingua**: GUI con auto-rilevamento locale (Italiano/Inglese)
- **✅ Verifica Integrità**: Controllo file compressi per validazione
- **🖥️ Dual Interface**: CLI potente e GUI user-friendly

## 🎬 Interfacce

### CLI (Command Line Interface)
```bash
# Comprime un file
file_compressor compress document.txt --livello 10

# Decomprime
file_compressor decompress document.txt.zst

# Comprime directory in archivio tar.zst
file_compressor compress my_folder --livello 5

# Multi-file in archivio
file_compressor multicompress file1.txt file2.txt --output backup.tar.zst

# Batch con pattern glob
file_compressor batch "**/*.log" --livello 3 --parallel

# Verifica integrità
file_compressor verifica archive.zst
```

### GUI (Graphical User Interface)
```bash
file_compressor_gui
```

Interfaccia drag-and-drop con:
- Selezione multipla file/directory
- Configurazione livello compressione (1-21)
- Compressione parallela
- Output personalizzato
- Dettagli operazioni

## 📦 Installazione

### Download Binari Pre-compilati

Scarica l'ultima release per la tua piattaforma:

- **Linux**: `FileCompressor-Linux-x86_64.AppImage`
- **Windows**: `FileCompressor-Windows-x86_64.zip`
- **macOS**: `FileCompressor-macOS-Universal.dmg` (Intel + Apple Silicon)

[📥 Download dalla pagina Releases](https://github.com/edocico/file_compressor/releases)

### Compilazione da Sorgente

Requisiti: Rust 1.70+

```bash
# Clone repository
git clone https://github.com/edocico/file_compressor.git
cd file_compressor

# Build release
cargo build --release

# Binari disponibili in:
# - target/release/file_compressor (CLI)
# - target/release/file_compressor_gui (GUI)
```

#### Dipendenze Linux
```bash
sudo apt-get install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
                     libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

## 🎮 Utilizzo

### CLI - Comandi

#### `compress` - Comprimi file o directory
```bash
file_compressor compress <FILE> [OPTIONS]

Options:
  -l, --livello <1-21>     Livello compressione (default: 3)
  -f, --force              Sovrascrivi file esistenti
  -p, --parallel           Usa compressione multi-threaded
  -o, --output <PATH>      Percorso destinazione
```

**Esempi:**
```bash
# Compressione veloce
file_compressor compress document.pdf --livello 1

# Compressione massima con parallelismo
file_compressor compress large_file.dat --livello 21 --parallel

# Comprimi directory
file_compressor compress project_folder/
```

#### `decompress` - Decomprimi file
```bash
file_compressor decompress <FILE> [OPTIONS]

Options:
  -f, --force         Sovrascrivi file esistenti
  -o, --output <PATH> Percorso destinazione
```

#### `multicompress` - Archivio multi-file
```bash
file_compressor multicompress <FILES...> --output archive.tar.zst [OPTIONS]
```

#### `batch` - Batch processing
```bash
file_compressor batch <PATTERN> [OPTIONS]

Examples:
  file_compressor batch "*.log" --livello 5
  file_compressor batch "**/*.txt" --parallel
```

#### `verifica` - Verifica integrità
```bash
file_compressor verifica <FILE>
```

### Livelli di Compressione

| Livello | Velocità | Ratio | Utilizzo Consigliato |
|---------|----------|-------|----------------------|
| 1-3     | ⚡ Veloce | 📦 Basso | File temporanei, backup rapidi |
| 4-9     | ⚖️ Bilanciato | 📦📦 Medio | Uso generale |
| 10-15   | 🐢 Lento | 📦📦📦 Alto | Archivi, distribuzione |
| 16-21   | 🐌 Molto lento | 📦📦📦📦 Massimo | Storage long-term |

## 🔧 Caratteristiche Tecniche

### Ottimizzazioni Automatiche

- **Buffer Adattivi**: 256KB per file <10MB, 1MB per file ≥10MB
- **Multi-threading Automatico**: File ≥1MB usano compressione parallela (se auto-parallel abilitato)
- **Ottimizzazioni File Grandi**: File ≥10MB abilitano WindowLog(24) e long-distance matching
- **Validazione Path**: Protezione contro directory traversal

### Architettura

```
file_compressor/
├── src/
│   ├── lib.rs       # Core library: compressione, decompressione, verifica
│   ├── main.rs      # CLI application
│   └── gui.rs       # GUI application (egui)
├── Cargo.toml
├── README.md
└── CLAUDE.md        # Developer guide
```

## 🧪 Test

```bash
# Esegui tutti i test
cargo test

# Test con output dettagliato
cargo test -- --nocapture

# Test specifico
cargo test test_compress_decompress_roundtrip
```

Coverage attuale: **35 test**, 100% passing ✅

## 🤝 Contribuire

I contributi sono benvenuti! Per favore:

1. Fork del repository
2. Crea un branch per la feature (`git checkout -b feature/amazing-feature`)
3. Commit delle modifiche (`git commit -m 'Add amazing feature'`)
4. Push al branch (`git push origin feature/amazing-feature`)
5. Apri una Pull Request

Assicurati che:
- I test passino: `cargo test`
- Il codice sia formattato: `cargo fmt`
- Non ci siano warning clippy: `cargo clippy -- -D warnings`

## 📝 Licenza

Questo progetto è dual-licensed sotto:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

Puoi scegliere la licenza che preferisci.

## 🙏 Ringraziamenti

- [Zstandard](https://facebook.github.io/zstd/) - Algoritmo di compressione
- [egui](https://github.com/emilk/egui) - GUI framework
- [clap](https://github.com/clap-rs/clap) - CLI parsing
- [rayon](https://github.com/rayon-rs/rayon) - Parallelismo

## 📬 Supporto

Per bug report e feature request, apri una [issue](https://github.com/edocico/file_compressor/issues).

---

Made with ❤️ in Rust
