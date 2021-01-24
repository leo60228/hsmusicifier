#!/usr/bin/env bash
set -ex

BUILD_DIR="$(mktemp -p /tmp -d appimage-build-XXXXXX)"

cleanup() {
    if [ -d "$BUILD_DIR" ] && [ -z "$PRESERVE" ]; then
        rm -rf "$BUILD_DIR"
    fi
}
trap cleanup EXIT

REPO_ROOT="$(readlink -f "$(dirname "$0")/..")"
OLD_CWD="$(readlink -f .)"

pushd "$BUILD_DIR"

cargo install --path="$REPO_ROOT" --root=.

wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage

chmod +x linuxdeploy-x86_64.AppImage
$APPIMAGE_RUN ./linuxdeploy-x86_64.AppImage \
    --appdir "$PWD/AppDir" \
    --executable "$PWD/bin/hsmusicifier" \
    --desktop-file "$REPO_ROOT/dist/hsmusicifier.desktop" \
    --icon-file "$REPO_ROOT/dist/hsmusicifier.png" \
    --output appimage

mv hsmusicifier-x86_64.AppImage "$OLD_CWD"
