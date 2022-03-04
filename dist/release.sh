#!/usr/bin/env bash
set -ex

appimage_add="$(readlink -f "$(dirname "$0")/appimage-add.sh")"
windows_zip="$(readlink -f "$1")"
linux_zip="$(readlink -f "$2")"
appimage_rt="$(readlink -f "$3")"
bandcamp_json="$(readlink -f "$4")"
hsmusic_data="$(readlink -f "$5")"
hsmusic_media="$(readlink -f "$6")"

cd "$(dirname "$0")/../target"
rm -rf dist
mkdir dist
cd dist

mkdir bin
pushd bin
unzip "$windows_zip"
unzip "$linux_zip"
popd

mkdir res
pushd res
cp "$bandcamp_json" .

mkdir hsmusic-data
cp -r "$hsmusic_data/album" hsmusic-data/

mkdir hsmusic-media
cp -r "$hsmusic_media/album-art" hsmusic-media/

zip -r ../hsmusicifier-data.zip *
popd

mkdir hsmusicifier-win
pushd hsmusicifier-win
cp ../bin/hsmusicifier.exe .
cp -r ../res/* .
popd

zip -r hsmusicifier-win.zip hsmusicifier-win/

"$appimage_add" bin/hsmusicifier-x86_64.AppImage hsmusicifier-x86_64.AppImage "$appimage_rt" res/*

mkdir out/
mv -v hsmusicifier-x86_64.AppImage out/
mv -v hsmusicifier-win.zip out/
mv -v hsmusicifier-data.zip out/
mv -v bin/hsmusicifier.exe out/
