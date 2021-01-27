#!/usr/bin/env bash
set -ex

appimage_add="$(readlink -f "$(dirname "$0")/appimage-add.sh")"
windows_zip="$(readlink -f "$1")"
ffmpeg="$(readlink -f "$2")"
linux_zip="$(readlink -f "$3")"
appimage_rt="$(readlink -f "$4")"
bandcamp_json="$(readlink -f "$5")"
hsmusic="$(readlink -f "$6")"

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

mkdir hsmusic
pushd hsmusic
mkdir media
cp -r "$hsmusic/media/album-art" media/
mkdir data
cp -r "$hsmusic/data/album" data/
popd

zip -r ../hsmusicifier-data.zip *
popd

mkdir hsmusicifier-win
pushd hsmusicifier-win
cp ../bin/hsmusicifier.exe .
cp -r ../res/* .
cp "$ffmpeg"/bin/*.dll .
popd

zip -r hsmusicifier-win.zip hsmusicifier-win/

"$appimage_add" bin/hsmusicifier-x86_64.AppImage hsmusicifier-x86_64.AppImage "$appimage_rt" res/*

mkdir out/
mv -v hsmusicifier-x86_64.AppImage out/
mv -v hsmusicifier-win.zip out/
mv -v hsmusicifier-data.zip out/
mv -v bin/hsmusicifier.exe out/
