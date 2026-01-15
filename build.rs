// Build script per embedding dell'icona su Windows
fn main() {
    // Solo per Windows
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;

        let icon_path = Path::new("assets/icon.ico");

        if icon_path.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            res.set("ProductName", "File Compressor");
            res.set("FileDescription", "Compressore e decompressore file con Zstandard");
            res.set("LegalCopyright", "Copyright (c) 2024");

            if let Err(e) = res.compile() {
                eprintln!("Errore durante la compilazione delle risorse Windows: {}", e);
            }
        } else {
            println!("cargo:warning=Icon file not found at assets/icon.ico - building without icon");
        }
    }
}
