#!/bin/bash
set -e

echo "=== Creazione AppImage per File Compressor ==="

# Compila in release
echo "[1/5] Compilazione..."
cargo build --release

# Crea struttura AppDir
echo "[2/5] Creazione struttura AppDir..."
rm -rf AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps

# Copia binario
cp target/release/file_compressor_gui AppDir/usr/bin/

# Copia icona
cp assets/icon.svg AppDir/usr/share/icons/hicolor/scalable/apps/file-compressor.svg
cp assets/icon.svg AppDir/file-compressor.svg

# Crea .desktop
cat > AppDir/file-compressor.desktop << 'EOF'
[Desktop Entry]
Name=File Compressor
Comment=Compressore/Decompressore Zstandard
Exec=file_compressor_gui
Icon=file-compressor
Terminal=false
Type=Application
Categories=Utility;Compression;
EOF

cp AppDir/file-compressor.desktop AppDir/usr/share/applications/

# Crea AppRun
cat > AppDir/AppRun << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
exec "${HERE}/usr/bin/file_compressor_gui" "$@"
EOF
chmod +x AppDir/AppRun

# Scarica appimagetool se non presente
echo "[3/5] Preparazione appimagetool..."
if [ ! -f appimagetool ]; then
    wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" -O appimagetool
    chmod +x appimagetool
fi

# Crea AppImage
echo "[4/5] Creazione AppImage..."
ARCH=x86_64 ./appimagetool --appimage-extract-and-run AppDir FileCompressor-x86_64.AppImage

# Pulizia
echo "[5/5] Pulizia..."
rm -rf AppDir

echo ""
echo "=== Fatto! ==="
echo "AppImage creato: FileCompressor-x86_64.AppImage"
echo ""
echo "Per usarlo:"
echo "  chmod +x FileCompressor-x86_64.AppImage"
echo "  ./FileCompressor-x86_64.AppImage"
echo ""
echo "Oppure doppio click dal file manager!"
