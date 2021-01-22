{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell {
  buildInputs = [ pkgconfig gtk3 gsettings-desktop-schemas ];

  GSETTINGS_DESKTOP_SCHEMAS = "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}";
}
