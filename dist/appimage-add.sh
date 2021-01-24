#!/usr/bin/env bash
set -e

appimage_in="$1"
appimage_out="$2"
appimage_runtime="$3"

if ! [ -f "$appimage_in" ]; then
    echo "appimage_in missing!" >&2
    exit 1
fi

if [ -z "$appimage_out" ]; then
    echo "appimage_out missing!" >&2
    exit 1
fi

if ! [ -f "$appimage_runtime" ]; then
    echo "appimage_runtime missing!" >&2
    exit 1
fi

shift 3

appimage-run -x hsmusicifier.AppDir "$appimage_in"

cp -r "$@" hsmusicifier.AppDir/usr/share/

mksquashfs hsmusicifier.AppDir hsmusicifier.squashfs -root-owned -noappend
cat "$appimage_runtime" hsmusicifier.squashfs > "$appimage_out"
chmod a+x "$appimage_out"

rm -rf hsmusicifier.AppDir hsmusicifier.squashfs
