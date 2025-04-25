{
  description = "Build cross-platform holochain apps and runtimes";

  inputs = {
    holonix.url = "github:holochain/holonix/main-0.5";
    nixpkgs.follows = "holonix/nixpkgs";
    rust-overlay.follows = "holonix/rust-overlay";
    crane.follows = "holonix/crane";

    holochain-nix-builders.url =
      "github:darksoil-studio/holochain-nix-builders/main-0.5";
    scaffolding.url = "github:darksoil-studio/scaffolding/main-0.5";
  };

  nixConfig = {
    extra-substituters = [
      "https://holochain-ci.cachix.org"
      "https://darksoil-studio.cachix.org"
    ];
    extra-trusted-public-keys = [
      "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8="
      "darksoil-studio.cachix.org-1:UEi+aujy44s41XL/pscLw37KEVpTEIn8N/kn7jO8rkc="
    ];
  };

  outputs = inputs@{ ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      flake = {
        lib = rec {
          filterTauriSources = { lib }:
            orig_path: type:
            let
              path = (toString orig_path);
              base = baseNameOf path;
              parentDir = baseNameOf (dirOf path);

              matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) [
                # Keep rust sources
                ".rs"
                # Keep all toml files as they are commonly used to configure other
                # cargo-based tools
                ".toml"
                # Keep icons
                ".png"
                # Keep tauri.conf.json and capabilities
                ".json"
              ];

              # Cargo.toml already captured above
              isCargoFile = base == "Cargo.lock";
              isSignerFile = base == "zome-call-signer.js";

              # .cargo/config.toml already captured above
              isCargoConfig = parentDir == ".cargo" && base == "config";
            in type == "directory" || matchesSuffix || isCargoFile
            || isCargoConfig || isSignerFile;
          cleanTauriSource = { lib }:
            src:
            lib.cleanSourceWith {
              src = lib.cleanSource src;
              filter = filterTauriSources { inherit lib; };

              name = "tauri-workspace";
            };
        };
      };

      imports = [
        ./crates/scaffold-tauri-happ/default.nix
        ./crates/scaffold-holochain-runtime/default.nix
        ./crates/hc-pilot/default.nix
        ./nix/tauri-cli.nix
        ./nix/android.nix
        # inputs.holochain-nix-builders.outputs.flakeModules.builders
        inputs.holochain-nix-builders.outputs.flakeModules.dependencies
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, self', pkgs, system, lib, ... }: rec {
        dependencies.tauriApp = let
          buildInputs = (with pkgs;
            [
              # openssl
              # openssl_3
            ]) ++ (lib.optionals pkgs.stdenv.isLinux (with pkgs; [
              # this is required for glib-networking
              glib
              webkitgtk # Brings libwebkit2gtk-4.0.so.37
              # webkitgtk.dev
              webkitgtk_4_1 # Needed for javascriptcoregtk
              # webkitgtk_4_1.dev
              # webkitgtk_6_0
              gdk-pixbuf
              gtk3
              # glib
              # stdenv.cc.cc.lib
              # harfbuzz
              # harfbuzzFull
              # zlib
              # xorg.libX11
              # xorg.libxcb
              # fribidi
              # fontconfig
              # freetype
              # libgpg-error
              # mesa
              # libdrm
              # libglvnd
              # Video/Audio data composition framework tools like "gst-inspect", "gst-launch" ...
              gst_all_1.gstreamer
              # Common plugins like "filesrc" to combine within e.g. gst-launch
              gst_all_1.gst-plugins-base
              # Specialized plugins separated by quality
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad
              gst_all_1.gst-plugins-ugly
              # Plugins to reuse ffmpeg to play almost every video format
              gst_all_1.gst-libav
              # Support the Video Audio (Hardware) Acceleration API
              gst_all_1.gst-vaapi
              libsoup_3
              dbus
              librsvg
            ])) ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs; [
              basez
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.CoreServices
              darwin.apple_sdk.frameworks.CoreFoundation
              darwin.apple_sdk.frameworks.Foundation
              darwin.apple_sdk.frameworks.AppKit
              darwin.apple_sdk.frameworks.WebKit
              darwin.apple_sdk.frameworks.Cocoa
            ]);
          nativeBuildInputs = (with pkgs; [ perl pkg-config makeWrapper ])
            ++ (lib.optionals pkgs.stdenv.isLinux
              (with pkgs; [ wrapGAppsHook ]))
            ++ (lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ]);
        in { inherit buildInputs nativeBuildInputs; };

        dependencies.tauriHapp = {
          buildInputs = dependencies.tauriApp.buildInputs
            ++ inputs.holochain-nix-builders.outputs.dependencies.${system}.holochain.buildInputs;
          nativeBuildInputs = dependencies.tauriApp.nativeBuildInputs;
        };

        devShells.tauriDev = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_20
            packages.tauriRust
            shared-mime-info
            gsettings-desktop-schemas
          ];

          buildInputs = dependencies.tauriApp.buildInputs;

          nativeBuildInputs = dependencies.tauriApp.nativeBuildInputs;

          shellHook = if pkgs.stdenv.isLinux then ''
            export GIO_MODULE_DIR=${pkgs.glib-networking}/lib/gio/modules/
            export GIO_EXTRA_MODULES=${pkgs.glib-networking}/lib/gio/modules
            export WEBKIT_DISABLE_COMPOSITING_MODE=1

            export XDG_DATA_DIRS=${pkgs.shared-mime-info}/share:${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
            unset SOURCE_DATE_EPOCH
          '' else ''
            export PATH=${pkgs.basez}/bin:$PATH
          '';

        };

        packages.tauriRust = let
          rust = inputs.holonix.packages.${system}.rust.override {
            extensions = [ "rust-src" ];
          };
          linuxCargo = pkgs.writeShellApplication {
            name = "cargo";
            runtimeInputs = [ rust ];
            text = ''
              RUSTFLAGS="-C link-arg=$(gcc -print-libgcc-file-name)" cargo "$@"
            '';
          };
        in if pkgs.stdenv.isLinux then linuxCargo else rust;

        packages.holochainTauriRust = let
          rust = inputs.holonix.packages.${system}.rust.override {
            extensions = [ "rust-src" ];
            targets = [ "wasm32-unknown-unknown" ];
          };
          linuxCargo = pkgs.writeShellApplication {
            name = "cargo";
            runtimeInputs = [ rust ];
            text = ''
              RUSTFLAGS="-C link-arg=$(gcc -print-libgcc-file-name)" cargo "$@"
            '';
          };
        in if pkgs.stdenv.isLinux then linuxCargo else rust;

        devShells.holochainTauriDev = pkgs.mkShell {
          inputsFrom = [
            devShells.tauriDev
            inputs'.holochain-nix-builders.devShells.holochainDev
          ];
          packages = [ packages.holochainTauriRust ];

          shellHook = ''
            export PS1='\[\033[1;34m\][p2p-shipyard:\w]\$\[\033[0m\] '
          '';
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ devShells.holochainTauriDev ];
          packages = [ pkgs.pnpm ];
        };
      };
    };
}
