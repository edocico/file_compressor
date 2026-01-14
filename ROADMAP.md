# PIANO IMPLEMENTAZIONE FASI 2-3: OTTIMIZZAZIONI AVANZATE
## File Compressor - Roadmap Completa

---

## ✅ FASE 1: COMPLETATA (Quick Wins)

### Implementato:
- ✅ **FileType detection** basato su estensione
- ✅ **Entropy calculation** per rilevare file già compressi/encrypted
- ✅ **Smart skip** per file multimediali e archivi
- ✅ **Content-based optimization** (preparato per future estensioni)
- ✅ **Parameter tuning avanzato**:
  - WindowLog dinamico (24-27) in base al livello
  - HashLog(26) e ChainLog(27) per file >100MB
  - Pledged source size per +2-5% ratio
- ✅ **CLI flag** `--no-smart` per disabilitare ottimizzazioni
- ✅ **4 nuovi test** (39 totali, 100% passing)

### Benefici misurati:
- Evita compressione inutile di file già compressi
- +2-5% ratio grazie a pledged_src_size
- Parametri ottimizzati per file grandi (>100MB)

---

## 📋 FASE 2: DICTIONARY TRAINING & BATCH OPTIMIZATION

**Durata stimata**: 2-3 giorni
**Difficoltà**: ⭐⭐⭐
**Beneficio**: +10-30% ratio su file omogenei

### Obiettivi:
1. Implementare dictionary training per file simili
2. Auto-detect pattern simili in batch operations
3. Cache dizionari per riuso
4. Integrazione con batch/multicompress

### Implementazione dettagliata:

#### 2.1 Dictionary Builder (lib.rs)
```rust
/// Struttura per gestire dizionari custom
pub struct CompressionDictionary {
    data: Vec<u8>,
    id: String, // Hash per identificazione
}

impl CompressionDictionary {
    /// Train dictionary da un set di file sample
    pub fn train_from_files(
        sample_files: &[PathBuf],
        dict_size: usize,
    ) -> std::io::Result<Self> {
        // 1. Leggi samples (max 100 file, 64KB ciascuno)
        // 2. Usa zstd::dict::from_samples()
        // 3. Genera ID univoco (hash SHA256)
        // 4. Ritorna dictionary
    }

    /// Train dictionary da buffer
    pub fn train_from_samples(
        samples: &[&[u8]],
        dict_size: usize,
    ) -> Result<Self, std::io::Error> {
        // Usa zstd::dict::from_samples direttamente
    }

    /// Salva dictionary su disco
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        // Serializza in formato zstd .dict
    }

    /// Carica dictionary da disco
    pub fn load(path: &Path) -> std::io::Result<Self> {
        // Deserializza
    }
}
```

#### 2.2 Dictionary Cache (lib.rs)
```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cache globale per dizionari
pub struct DictionaryCache {
    cache: Arc<RwLock<HashMap<String, CompressionDictionary>>>,
    cache_dir: PathBuf, // ~/.cache/file_compressor/dicts/
}

impl DictionaryCache {
    pub fn new() -> Self {
        // Crea cache_dir se non esiste
    }

    /// Ottieni dictionary per un set di file
    pub fn get_or_create(
        &self,
        files: &[PathBuf],
        dict_size: usize,
    ) -> std::io::Result<CompressionDictionary> {
        // 1. Calcola signature dei file (hash concatenati)
        // 2. Controlla se esiste in cache
        // 3. Se no, train nuovo dictionary
        // 4. Salva su disco e in memoria
        // 5. Ritorna dictionary
    }

    /// Pulisci cache (rimuovi dict > 30 giorni)
    pub fn cleanup(&self) -> std::io::Result<()> {
        // Scansiona cache_dir e rimuovi vecchi
    }
}
```

#### 2.3 Integrazione CompressOptions (lib.rs)
```rust
pub struct CompressOptions {
    // ... campi esistenti ...
    pub use_dictionary: bool,
    pub dictionary: Option<CompressionDictionary>,
    pub auto_train_dict: bool, // Train automatico per batch
}

impl CompressOptions {
    pub fn with_dictionary(mut self, dict: CompressionDictionary) -> Self {
        self.dictionary = Some(dict);
        self.use_dictionary = true;
        self
    }

    pub fn with_auto_train_dict(mut self, enable: bool) -> Self {
        self.auto_train_dict = enable;
        self
    }
}
```

#### 2.4 Modifica compress_file per dictionary (lib.rs)
```rust
// In compress_file(), dopo creazione encoder:
if let Some(dict) = &options.dictionary {
    encoder.with_dictionary(&dict.data)?;
}
```

#### 2.5 CLI commands (main.rs)
```bash
# Nuovo subcommand
Commands::TrainDict {
    /// Sample files per training
    #[arg(value_name = "FILES", num_args = 1..)]
    sample_files: Vec<PathBuf>,

    /// Output dictionary file
    #[arg(short, long, default_value = "custom.dict")]
    output: PathBuf,

    /// Dictionary size (default: 110KB)
    #[arg(short, long, default_value = "112640")]
    size: usize,
}

# Flag per batch/multicompress
Batch {
    // ... campi esistenti ...

    /// Usa dictionary auto-training
    #[arg(long)]
    auto_dict: bool,

    /// Usa dictionary esistente
    #[arg(short = 'd', long, value_name = "FILE")]
    dict_file: Option<PathBuf>,
}
```

#### 2.6 Batch optimization con dictionary (main.rs)
```rust
fn batch_compress_with_dict(
    pattern: &str,
    level: i32,
    force: bool,
    parallel: bool,
    auto_dict: bool,
) -> std::io::Result<()> {
    let files: Vec<PathBuf> = /* glob */;

    let dict = if auto_dict {
        // Sample 10-20% dei file per training
        let samples: Vec<PathBuf> = files
            .iter()
            .step_by(files.len() / 20.max(1))
            .take(20)
            .cloned()
            .collect();

        Some(CompressionDictionary::train_from_files(&samples, 110_000)?)
    } else {
        None
    };

    // Comprimi tutti con dictionary
    files.par_iter().for_each(|file| {
        let options = CompressOptions::new(level)
            .with_force(force)
            .with_dictionary(dict.clone());
        compress_file(file, &options)
    });
}
```

### Test da aggiungere:
```rust
#[test]
fn test_dictionary_training() {
    // Crea 10 file JSON simili
    // Train dictionary
    // Verifica miglioramento ratio (almeno +10%)
}

#[test]
fn test_dictionary_cache() {
    // Train, salva, ricarica
    // Verifica ID matching
}

#[test]
fn test_batch_with_dict() {
    // Batch compress con auto-dict
    // Verifica ratio migliorato
}
```

### Metriche successo:
- ✅ +15-30% ratio su file JSON/log omogenei
- ✅ +10-20% ratio su backup incrementali
- ✅ Cache hit >80% su operazioni ripetute

---

## 🚀 FASE 3: MULTI-ALGORITHM SUPPORT

**Durata stimata**: 3-4 giorni
**Difficoltà**: ⭐⭐⭐⭐
**Beneficio**: Uso ottimale per ogni scenario

### Obiettivi:
1. Supporto LZ4 (velocità massima)
2. Supporto Brotli (testo/web)
3. Auto-selection basata su file type e priority
4. Benchmark integrato

### 3.1 Enum CompressionAlgorithm (lib.rs)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    Zstd,
    Lz4,
    Brotli,
    Auto, // Scelta automatica
}

impl CompressionAlgorithm {
    /// Seleziona algoritmo ottimale per tipo file e priorità
    pub fn auto_select(
        file_type: FileType,
        priority: CompressionPriority,
    ) -> Self {
        match (file_type, priority) {
            // Velocità massima
            (_, CompressionPriority::Speed) => CompressionAlgorithm::Lz4,

            // Ratio massimo per testo
            (FileType::Text, CompressionPriority::Ratio) => {
                CompressionAlgorithm::Brotli
            }

            // Bilanciato (default)
            _ => CompressionAlgorithm::Zstd,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionPriority {
    Speed,    // Velocità > ratio
    Balanced, // Bilanciato (default)
    Ratio,    // Ratio > velocità
}
```

### 3.2 Dipendenze Cargo.toml
```toml
[dependencies]
zstd = "0.13"
lz4 = { version = "1.24", optional = true }
brotli = { version = "6.0", optional = true }

[features]
default = ["lz4", "brotli"]
lz4-support = ["lz4"]
brotli-support = ["brotli"]
```

### 3.3 Trait per compressori (lib.rs)
```rust
trait Compressor {
    fn compress(
        &self,
        input: &Path,
        output: &Path,
        options: &CompressOptions,
    ) -> std::io::Result<CompressionResult>;

    fn decompress(
        &self,
        input: &Path,
        output: &Path,
        options: &DecompressOptions,
    ) -> std::io::Result<()>;

    fn extension(&self) -> &'static str;
}

struct ZstdCompressor;
struct Lz4Compressor;
struct BrotliCompressor;

// Implementazioni per ciascuno
```

### 3.4 Factory pattern
```rust
pub fn get_compressor(
    algorithm: CompressionAlgorithm,
) -> Box<dyn Compressor> {
    match algorithm {
        CompressionAlgorithm::Zstd => Box::new(ZstdCompressor),
        #[cfg(feature = "lz4-support")]
        CompressionAlgorithm::Lz4 => Box::new(Lz4Compressor),
        #[cfg(feature = "brotli-support")]
        CompressionAlgorithm::Brotli => Box::new(BrotliCompressor),
        _ => panic!("Algorithm not supported"),
    }
}
```

### 3.5 CLI integration (main.rs)
```bash
Commands::Compress {
    // ... campi esistenti ...

    /// Algoritmo di compressione
    #[arg(short = 'a', long, value_parser = parse_algorithm)]
    algorithm: Option<CompressionAlgorithm>,

    /// Priorità: speed, balanced, ratio
    #[arg(short = 'P', long, default_value = "balanced")]
    priority: CompressionPriority,
}
```

### 3.6 Benchmark command (main.rs)
```rust
Commands::Benchmark {
    /// File di test
    #[arg(value_name = "FILE")]
    input_file: PathBuf,

    /// Test tutti gli algoritmi
    #[arg(short, long)]
    all: bool,

    /// Livelli da testare
    #[arg(short, long, default_value = "1,3,5,9,15")]
    levels: String,
}

fn benchmark_file(file: &Path, algorithms: &[CompressionAlgorithm]) {
    println!("| Algorithm | Level | Time | Ratio | Speed |");
    println!("|-----------|-------|------|-------|-------|");

    for algo in algorithms {
        for level in &[1, 3, 5, 9, 15] {
            let start = Instant::now();
            let result = compress_with_algo(file, algo, *level)?;
            let elapsed = start.elapsed();

            let speed_mb_s = (file_size as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
            let ratio = result.input_size as f64 / result.output_size as f64;

            println!(
                "| {:?} | {} | {:.2}s | {:.2}x | {:.0} MB/s |",
                algo, level, elapsed.as_secs_f64(), ratio, speed_mb_s
            );
        }
    }
}
```

### Test da aggiungere:
```rust
#[cfg(feature = "lz4-support")]
#[test]
fn test_lz4_compression() {
    // Comprimi con LZ4
    // Verifica velocità >2x di zstd
}

#[cfg(feature = "brotli-support")]
#[test]
fn test_brotli_on_text() {
    // Comprimi testo con Brotli
    // Verifica ratio migliore di zstd del 5-10%
}

#[test]
fn test_auto_algorithm_selection() {
    // Verifica scelta corretta per vari FileType
}
```

### Metriche successo:
- ✅ LZ4: 800+ MB/s compressione
- ✅ Brotli: +10% ratio su testo vs zstd
- ✅ Auto-select: scelta ottimale >90% casi

---

## 🔬 FASE 4: PRE-PROCESSING AVANZATO (Opzionale)

**Durata stimata**: 5-7 giorni
**Difficoltà**: ⭐⭐⭐⭐⭐
**Beneficio**: +5-15% ratio su file specifici

### 4.1 Delta encoding per database dumps
```rust
pub fn delta_encode(data: &[u8]) -> Vec<u8> {
    // Implementa delta encoding
    // Ottimo per time-series, database incrementali
}
```

### 4.2 BWT (Burrows-Wheeler Transform) per testo
```rust
pub fn bwt_transform(text: &str) -> Vec<u8> {
    // Implementa BWT
    // Migliora compressione testo del 5-15%
}
```

### 4.3 Solid compression (7z-style)
```rust
pub fn compress_solid(files: &[PathBuf]) -> Result<()> {
    // Comprimi come stream unico
    // +10-20% ratio su file piccoli
}
```

---

## 📊 FASE 5: MONITORING & ANALYTICS

**Durata stimata**: 2 giorni
**Difficoltà**: ⭐⭐

### 5.1 Statistiche compressione
```rust
pub struct CompressionStats {
    pub total_files: usize,
    pub total_input_size: u64,
    pub total_output_size: u64,
    pub total_time: Duration,
    pub avg_ratio: f64,
    pub avg_speed_mb_s: f64,
}
```

### 5.2 History tracking
```bash
# Nuovo comando
file_compressor history
# Mostra ultimi 50 comandi con statistiche

file_compressor stats
# Mostra statistiche aggregate
```

---

## 🎯 TIMELINE COMPLESSIVA

| Fase | Durata | Priorità | Dipendenze |
|------|--------|----------|------------|
| ✅ Fase 1 | COMPLETATA | ⭐⭐⭐⭐⭐ | - |
| Fase 2 | 2-3 giorni | ⭐⭐⭐⭐ | Fase 1 |
| Fase 3 | 3-4 giorni | ⭐⭐⭐ | Fase 1 |
| Fase 4 | 5-7 giorni | ⭐⭐ | Fase 2, 3 |
| Fase 5 | 2 giorni | ⭐⭐ | Qualsiasi |

**Totale: 12-16 giorni** per implementazione completa

---

## 🏆 BENEFICI ATTESI (Cumulativi)

| Fase | Beneficio | Casi d'uso principali |
|------|-----------|----------------------|
| Fase 1 | +2-5% ratio, skip file compressi | Tutti |
| Fase 2 | +10-30% ratio su file omogenei | Log aggregation, backup, dataset |
| Fase 3 | Velocità ottimale per scenario | Cache, archivi, distribuzione |
| Fase 4 | +5-15% ratio su file specifici | Database dumps, time-series |
| Fase 5 | Monitoring e insights | Operations, debugging |

### Scenario ottimale finale:
- **Log server**: Fase 1 + 2 → +25-35% ratio totale
- **Backup incrementali**: Fase 1 + 2 + 4 → +30-45% ratio totale
- **Cache applicazioni**: Fase 1 + 3 (LZ4) → 5x più veloce
- **Distribuzione software**: Fase 1 + 3 (Brotli) → +15% ratio

---

## 📝 NOTE IMPLEMENTAZIONE

### Best Practices:
1. ✅ Feature flags per algoritmi opzionali
2. ✅ Backward compatibility su formato file
3. ✅ Graceful degradation se feature non disponibile
4. ✅ Test coverage >80% per nuove features
5. ✅ Benchmark prima/dopo ogni fase

### Breaking Changes da evitare:
- ❌ Non cambiare formato .zst di default
- ❌ Non rimuovere opzioni CLI esistenti
- ✅ Usare flag opt-in per nuove features
- ✅ Mantenere API backward compatible

---

## 🚦 PROSSIMI PASSI IMMEDIATI

1. **Commit Fase 1** ✅
2. **Inizia Fase 2**:
   - `cargo add` zstd con feature dict
   - Implementa `CompressionDictionary` struct
   - Test dictionary training su 10 file JSON
3. **Parallelo a Fase 2**:
   - Documenta Fase 1 in README.md
   - Crea issue GitHub per Fase 2-5

---

Questo piano è modulare: ogni fase è indipendente e porta valore incrementale! 🎉
