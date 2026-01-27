#!/bin/bash
set -e

APP_NAME="KoShelf"
APP_DIR="release/${APP_NAME}.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"

echo "🔨 Building macOS App Bundle with Auto-Config Wrapper..."
mkdir -p "$MACOS_DIR"

# 1. Criar o Wrapper (Lançador) em vez de copiar o binário direto para o topo
cat > "$MACOS_DIR/launcher.sh" <<EOF
#!/bin/bash
APP_DATA_DIR="\$HOME/Documents/KoShelf_Library"
mkdir -p "\$APP_DATA_DIR"
mkdir -p "\$HOME/Books"

# Entra na pasta de Documentos para que o settings.json e o site 
# sejam criados onde o macOS permite escrita.
cd "\$APP_DATA_DIR"

# Inicia o servidor na porta 3000
"\$(dirname "\$0")/koshelf_bin" --library-path "\$HOME/Books" --port 3000 "\$@"
EOF

chmod +x "$MACOS_DIR/launcher.sh"

# 2. Copiar o binário real com um nome interno
if [ -f "target/release/koshelf" ]; then
    cp target/release/koshelf "$MACOS_DIR/koshelf_bin"
else
    echo "❌ Error: Binary not found."
    exit 1
fi

# 3. Info.plist apontando para o launcher.sh
cat > "${CONTENTS_DIR}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>launcher.sh</string>
    <key>CFBundleIdentifier</key>
    <string>com.zanivann.koshelf</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.8.4</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
</dict>
</plist>
EOF

chmod +x "$MACOS_DIR/koshelf_bin"