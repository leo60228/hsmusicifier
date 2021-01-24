{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell {
  buildInputs = [ pkgconfig gtk3 gsettings-desktop-schemas ffmpeg squashfsTools zip wrapGAppsHook (appimage-run.override {
    extraPkgs = pkgs: with pkgs; [ gmp6 ];
  }) ] ++ ffmpeg.buildInputs;

  GSETTINGS_DESKTOP_SCHEMAS = "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}";

  APPIMAGE_RUN = "${appimage-run}/bin/appimage-run";
}
