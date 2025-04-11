{
  description = "Build cross-platform holochain apps and runtimes";

  inputs = {
    holonix.url = "github:holochain/holonix";
    nixpkgs.follows = "holonix/nixpkgs";
    rust-overlay.follows = "holonix/rust-overlay";
    crane.follows = "holonix/crane";

    tnesh-stack.url = "github:darksoil-studio/tnesh-stack/main-0.5";

    android-nixpkgs.url = "github:tadfisher/android-nixpkgs";
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
          # tauriAppDeps = 
          #   libraries = { pkgs, lib }:
          #     with pkgs; [
          #       (customGlib pkgs)
          #       webkitgtk
          #       webkitgtk_4_1
          #       # gtk3
          #       # cairo
          #       # gdk-pixbuf
          #       # glib
          #       # # glib.dev
          #       # dbus
          #       # # openssl_3
          #       # librsvg
          #       # harfbuzz
          #       # harfbuzzFull
          #       # stdenv.cc.cc.lib
          #       # zlib
          #       # xorg.libX11
          #       # xorg.libxcb
          #       # fribidi
          #       # fontconfig
          #       # freetype
          #       # libgpg-error
          #       # mesa
          #       # libdrm
          #       # libglvnd
          #     ];
          # };
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
        ./nix/modules/tauri-cli.nix
        # inputs.tnesh-stack.outputs.flakeModules.builders
        inputs.tnesh-stack.outputs.flakeModules.dependencies
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, self', pkgs, system, lib, ... }:
        let
          getNdk = sdkPkgs: sdkPkgs.ndk-25-0-8775105;
          ndkFolder = "ndk/25.0.8775105";
        in rec {
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
              ++ inputs.tnesh-stack.outputs.dependencies.${system}.holochain.buildInputs;
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

            # WIP testing stuff to attempt to have npm tauri build work inside of nix shells
            # export PKG_CONFIG_PATH=${
            #   flake.lib.tauriAppDeps.customGlib
            #   inputs'.webkitgtknixpkgs.legacyPackages
            # }/lib/pkgconfig:$PKG_CONFIG_PATH
            # export PATH=${
            #   flake.lib.tauriAppDeps.customCp
            #   inputs'.webkitgtknixpkgs.legacyPackages
            # }/bin:$PATH
          };

          packages.android-sdk = inputs.android-nixpkgs.sdk.${system} (sdkPkgs:
            with sdkPkgs; [
              cmdline-tools-latest
              build-tools-34-0-0
              build-tools-30-0-3
              platform-tools
              (getNdk sdkPkgs)
              platforms-android-34
            ]);

          devShells.androidDev = pkgs.mkShell {
            packages =
              [ packages.android-sdk pkgs.gradle pkgs.jdk17 pkgs.aapt ];

            shellHook = ''
              export GRADLE_OPTS="-Dorg.gradle.project.android.aapt2FromMavenOverride=${pkgs.aapt}/bin/aapt2";

              export ANDROID_SDK_ROOT=${packages.android-sdk}/share/android-sdk
              export ANDROID_HOME=${packages.android-sdk}/share/android-sdk
              export NDK_HOME=${packages.android-sdk}/share/android-sdk/${ndkFolder}
              export ANDROID_STANDALONE_TOOLCHAIN=$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64
              export CMAKE_TOOLCHAIN_FILE=$NDK_HOME/build/cmake/android.toolchain.cmake
            '';
          };

          devShells.androidEmulatorDev = let
            android-sdk = inputs.android-nixpkgs.sdk.${system} (sdkPkgs:
              with sdkPkgs; [
                cmdline-tools-latest
                build-tools-30-0-3
                platform-tools
                (getNdk sdkPkgs)
                platforms-android-34
                emulator
                system-images-android-34-google-apis-playstore-x86-64
              ]);
          in pkgs.mkShell {
            inputsFrom = [ devShells.androidDev ];
            packages = [ android-sdk ];

            shellHook = ''
              echo "no" | avdmanager -s create avd -n Pixel -k "system-images;android-34;google_apis_playstore;x86_64" --force
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
            linuxRust = pkgs.symlinkJoin {
              name = "rust";
              paths = [ linuxCargo rust ];
            };
          in if pkgs.stdenv.isLinux then linuxRust else rust;

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
            linuxRust = pkgs.symlinkJoin {
              name = "holochain-tauri-rust-for-linux";
              paths = [ linuxCargo rust ];
            };
          in if pkgs.stdenv.isLinux then linuxRust else rust;

          packages.androidTauriRust = let
            rust = inputs.holonix.packages.${system}.rust.override {
              extensions = [ "rust-src" ];
              targets = [
                "armv7-linux-androideabi"
                "x86_64-linux-android"
                "i686-linux-android"
                "aarch64-unknown-linux-musl"
                "wasm32-unknown-unknown"
                "x86_64-pc-windows-gnu"
                "x86_64-unknown-linux-musl"
                "x86_64-apple-darwin"
                "aarch64-linux-android"
              ];
            };
            linuxCargo = pkgs.writeShellApplication {
              name = "cargo";
              runtimeInputs = [ rust ];
              text = ''
                RUSTFLAGS="-C link-arg=$(gcc -print-libgcc-file-name)" cargo "$@"
              '';
            };
            # customZigbuildCargo = pkgs.writeShellApplication {
            #   name = "cargo";

            #   runtimeInputs = (lib.optionals pkgs.stdenv.isLinux [ linuxCargo ])
            #     ++ [
            #       rust
            #       (pkgs.callPackage ./nix/custom-cargo-zigbuild.nix { })
            #     ];

            #   text = ''
            #     if [ "$#" -ne 0 ] && [ "$1" = "build" ]
            #     then
            #       cargo-zigbuild "$@"
            #     else
            #       cargo "$@"
            #     fi
            #   '';
            # };
            androidRust = pkgs.symlinkJoin {
              name = "rust-for-android";
              paths = [
                # customZigbuildCargo
                # linuxCargo
                rust
                packages.android-sdk
              ];
              buildInputs = [ pkgs.makeWrapper ];
              postBuild = let
                ndkPath =
                  "${packages.android-sdk}/share/android-sdk/${ndkFolder}";
                prebuiltPath = "${ndkPath}/toolchains/llvm/prebuilt/${
                    if pkgs.stdenv.isLinux then
                      "linux-x86_64"
                    else
                      "darwin-x86_64"
                  }";
                toolchainBinsPath = "${prebuiltPath}/bin";
                # --set CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS "-L${prebuiltPath}/lib/clang/19/lib/linux -lstatic=clang_rt.builtins-aarch64-android" \
                # --set CARGO_TARGET_I686_LINUX_ANDROID_RUSTFLAGS "-L${prebuiltPath}/lib/clang/19/lib/linux -lstatic=clang_rt.builtins-i686-android" \
                # --set CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS "-L${prebuiltPath}/lib/clang/19/lib/linux -lstatic=clang_rt.builtins-arm-android" \
                # --set CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS "-L${prebuiltPath}/lib64/clang/14.0.7/lib/linux" \
              in ''
                wrapProgram $out/bin/cargo \
                  --set CFLAGS_AARCH64_LINUX_ANDROID "--target=aarch64-linux-android21" \
                  --set CXXFLAGS_AARCH64_LINUX_ANDROID "--target=aarch64-linux-android21" \
                  --set CFLAGS_X86_64_LINUX_ANDROID "--target=x86_64-linux-android21" \
                  --set CXXFLAGS_X86_64_LINUX_ANDROID "--target=x86_64-linux-android21" \
                  --set AR ${toolchainBinsPath}/llvm-ar \
                  --set CARGO_TARGET_AARCH64_LINUX_ANDROID_AR ${toolchainBinsPath}/llvm-ar \
                  --set CARGO_TARGET_I686_LINUX_ANDROID_AR ${toolchainBinsPath}/llvm-ar \
                  --set CARGO_TARGET_X86_64_LINUX_ANDROID_AR  ${toolchainBinsPath}/llvm-ar \
                  --set CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR  ${toolchainBinsPath}/llvm-ar \
                  --set RANLIB ${toolchainBinsPath}/llvm-ranlib \
                  --set CC_AARCH64_LINUX_ANDROID ${toolchainBinsPath}/aarch64-linux-android24-clang \
                  --set CXX_AARC64_LINUX_ANDROID ${toolchainBinsPath}/aarch64-linux-android24-clang++ \
                  --set CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER ${toolchainBinsPath}/aarch64-linux-android24-clang \
                  --set CC_I686_LINUX_ANDROID ${toolchainBinsPath}/i686-linux-android24-clang \
                  --set CXX_I686_LINUX_ANDROID ${toolchainBinsPath}/i686-linux-android24-clang++ \
                  --set CARGO_TARGET_I686_LINUX_ANDROID_LINKER ${toolchainBinsPath}/i686-linux-android24-clang \
                  --set CC_X86_64_LINUX_ANDROID ${toolchainBinsPath}/x86_64-linux-android24-clang \
                  --set CXX_X86_64_LINUX_ANDROID ${toolchainBinsPath}/x86_64-linux-android24-clang++ \
                  --set CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER ${toolchainBinsPath}/x86_64-linux-android24-clang \
                  --set CC_ARMV7_LINUX_ANDROIDEABI ${toolchainBinsPath}/armv7a-linux-androideabi24-clang \
                  --set CXX_ARMV7_LINUX_ANDROIDEABI ${toolchainBinsPath}/armv7a-linux-androideabi24-clang++ \
                  --set CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER ${toolchainBinsPath}/armv7a-linux-androideabi24-clang \
                  --set ANDROID_STANDALONE_TOOLCHAIN ${prebuiltPath} \
                  --set ANDROID_NDK ${ndkPath} \
                  --set ANDROID_NDK_HOME ${ndkPath} \
                  --set ANDROID_NDK_ROOT ${ndkPath} \
                  --set CMAKE_GENERATOR Ninja \
                  --set CMAKE_TOOLCHAIN_FILE ${ndkPath}/build/cmake/android.toolchain.cmake \
                  --set CLANG_PATH ${toolchainBinsPath}/clang \
                  --set CFLAGS "-isystem ${prebuiltPath}/include" \
                  --set LIBCLANG_PATH ${pkgs.llvmPackages_18.libclang.lib}/lib \
                  --set BINDGEN_EXTRA_CLANG_ARGS_AARCH64_LINUX_ANDROID "--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/aarch64-linux-android" \
                  --set BINDGEN_EXTRA_CLANG_ARGS_X86_64_LINUX_ANDROID "--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/x86_64-linux-android" \
                  --set BINDGEN_EXTRA_CLANG_ARGS_ARMV7_LINUX_ANDROIDEABI "--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/arm-linux-androideabi" \
                  --set BINDGEN_EXTRA_CLANG_ARGS_I686_LINUX_ANDROID "--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/i686-linux-android"
              '';
            };
          in androidRust;
          # --set BINDGEN_EXTRA_CLANG_ARGS_x86_64_linux_android "--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/x86_64-linux-android  -I${prebuiltPath}/sysroot/usr/include "
          # \
          #                 --set BINDGEN_EXTRA_CLANG_ARGS "-isystem ${prebuiltPath}/include" \
          devShells.holochainTauriDev = pkgs.mkShell {
            inputsFrom =
              [ devShells.tauriDev inputs'.tnesh-stack.devShells.holochainDev ];
            packages = [ packages.holochainTauriRust ];

            shellHook = ''
              export PS1='\[\033[1;34m\][p2p-shipyard:\w]\$\[\033[0m\] '
            '';
          };

          devShells.holochainTauriAndroidDev = pkgs.mkShell {
            inputsFrom = [ devShells.tauriDev devShells.androidDev ];
            packages = [ packages.androidTauriRust ];
            buildInputs =
              inputs.tnesh-stack.outputs.dependencies.${system}.holochain.buildInputs
              ++ (with pkgs;
                [
                  # glibc_multi
                  ninja
                ]);
            # nativeBuildInputs = [ pkgs.llvmPackages_18.libclang pkgs.ninja ];

            shellHook = ''

              export PS1='\[\033[1;34m\][p2p-shipyard-android:\w]\$\[\033[0m\] '
            '';
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ devShells.holochainTauriDev ];
            packages = [ inputs'.tnesh-stack.packages.synchronized-pnpm ];
          };
        };
    };
}
