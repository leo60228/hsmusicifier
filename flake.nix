{
  inputs.naersk.url = "github:leo60228/naersk/fetchgit-submodules";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.gitignore = {
    url = "github:hercules-ci/gitignore.nix";
    flake = false;
  };
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { nixpkgs, rust-overlay, naersk, gitignore, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlay ];
      };
      gitignore-lib = import gitignore { inherit (pkgs) lib; };
      inherit (gitignore-lib) gitignoreSource;
      rust = pkgs.rust-bin.nightly.latest.rust;
      devRust = rust.override {
        extensions = [ "rust-analyzer-preview" "rust-src" ];
      };
      makeNaersk = rust: naersk.lib.x86_64-linux.override {
        cargo = rust;
        rustc = rust;
      };
      buildNaersk = makeNaersk rust;
      devNaersk = makeNaersk devRust;
      makeHsmusicifier = buildPackage: buildPackage {
        root = gitignoreSource ./.;
        nativeBuildInputs = with pkgs; [ pkgconfig wrapGAppsHook git llvmPackages.llvm ];
        buildInputs = with pkgs; [ gtk3 gsettings-desktop-schemas ffmpeg zip openssl stdenv.cc.libc ];
        override = x: (x // {
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";
          preConfigure = ''
          export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.clang}/resource-root/include $NIX_CFLAGS_COMPILE"
          '';
        });
      };
    in rec {
      packages.hsmusicifier = makeHsmusicifier buildNaersk.buildPackage;
      defaultPackage = packages.hsmusicifier;

      devShell = with pkgs; mkShell {
        inputsFrom = (makeHsmusicifier devNaersk.buildPackage).builtDependencies ++ [ ffmpeg ];
        buildInputs = [ squashfsTools (appimage-run.override {
          extraPkgs = pkgs: with pkgs; [ gmp6 ];
        }) ];

        GSETTINGS_DESKTOP_SCHEMAS = "${gsettings-desktop-schemas}/share/gsettings-schemas/${gsettings-desktop-schemas.name}:${gtk3}/share/gsettings-schemas/${gtk3.name}";
        APPIMAGE_RUN = "${appimage-run}/bin/appimage-run";
      };
    }
  );
}
