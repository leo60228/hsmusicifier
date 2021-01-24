{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell {
  buildInputs = [ pkgconfig gtk3 gsettings-desktop-schemas ffmpeg appimage-run squashfsTools zip wrapGAppsHook ] ++ ffmpeg.buildInputs;

  GSETTINGS_DESKTOP_SCHEMAS = "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}";

  APPIMAGE_RUN = "${appimage-run}/bin/appimage-run";
}
