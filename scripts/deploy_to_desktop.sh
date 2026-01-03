#!/bin/bash
set -e

# Configuration
FINAL_APP_NAME="MrHedgehog"
BUILD_APP_NAME="MrHedgehogUI"
PROJECT_ROOT=$(pwd)
DESKTOP_PATH="$HOME/Desktop"
BUILD_DIR="$PROJECT_ROOT/frontend/build"
RUST_BIN="$PROJECT_ROOT/target/release/mr_hedgehog"

echo "🦔 Packaging $FINAL_APP_NAME for Desktop Deployment..."

# 1. Build Rust Backend (Release)
echo "🦀 Building Rust backend..."
cargo build --release

# 2. Build Qt Frontend
echo "🖥️ Building Qt frontend..."
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"
cmake ..
make -j$(sysctl -n hw.ncpu)

# 3. Bundle Preparation
echo "📦 Bundling application..."
APP_BUNDLE="$BUILD_DIR/${BUILD_APP_NAME}.app"

# Ensure Rust binary is inside the bundle
# macOS bundles look like: App.app/Contents/MacOS/{Binary}
cp "$RUST_BIN" "$APP_BUNDLE/Contents/MacOS/"
echo "✅ Copied mr_hedgehog to Bundle."

# 4. Qt Deployment
# Use macdeployqt to bundle Qt frameworks and plugins
if command -v macdeployqt &> /dev/null; then
    echo "🔧 Running macdeployqt..."
    # We add -libpath to help it find frameworks in /usr/local/lib
    macdeployqt "$APP_BUNDLE" -executable="$APP_BUNDLE/Contents/MacOS/mr_hedgehog" -libpath=/usr/local/lib -verbose=1
else
    echo "⚠️  Warning: macdeployqt not found. The app might not work on other machines."
fi

# 5. Move to Desktop
echo "🚀 Deploying to Desktop..."
TARGET_APP="$DESKTOP_PATH/${FINAL_APP_NAME}.app"

if [ -d "$TARGET_APP" ]; then
    echo "♻️  Replacing existing app on Desktop..."
    rm -rf "$TARGET_APP"
fi

cp -r "$APP_BUNDLE" "$TARGET_APP"

echo "✨ Done! You can now double-click '$FINAL_APP_NAME' on your Desktop."
echo "   Location: $TARGET_APP"
