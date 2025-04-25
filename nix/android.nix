{ inputs, ... }:

{
  perSystem = { inputs', lib, pkgs, self', system, ... }:
    let
      sdkPath = "${self'.packages.android-sdk}/libexec/android-sdk";
      ndkPath = "${sdkPath}/ndk-bundle";
      toolchainSystem =
        if pkgs.stdenv.isLinux then "linux-x86_64" else "darwin-x86_64";
      prebuiltPath = "${ndkPath}/toolchains/llvm/prebuilt/${toolchainSystem}";
      toolchainBinsPath = "${prebuiltPath}/bin";

    in rec {

      packages.android-sdk = let
        pkgs = import inputs.nixpkgs {
          inherit system;
          config.allowUnfree = true;
          config.android_sdk.accept_license = true;
        };
      in (pkgs.androidenv.composeAndroidPackages {
        platformVersions = [ "30" "34" ];
        buildToolsVersions = [ "30.0.3" ];
        systemImageTypes = [ "google_apis_playstore" ];
        abiVersions = [ "armeabi-v7a" "arm64-v8a" "x86" "x86_64" ];
        includeNDK = true;
        ndkVersion = "25.2.9519653";

        # includeExtras = [ "extras" "google" "auto" ];
      }).androidsdk;

      devShells.androidDev = pkgs.mkShell {
        packages = [ packages.android-sdk pkgs.gradle pkgs.jdk17 pkgs.aapt ];

        shellHook = ''
          export GRADLE_OPTS="-Dorg.gradle.project.android.aapt2FromMavenOverride=${pkgs.aapt}/bin/aapt2";

          export ANDROID_HOME=${sdkPath} 
          export ANDROID_SDK_ROOT=${sdkPath} 
          export ANDROID_NDK=${ndkPath} 
          export ANDROID_NDK_HOME=${ndkPath} 
          export ANDROID_NDK_ROOT=${ndkPath} 
          export NDK_HOME=${ndkPath}
          export ANDROID_NDK_LATEST_HOME=${ndkPath}

          export RANLIB=${toolchainBinsPath}/llvm-ranlib 
          export AR=${toolchainBinsPath}/llvm-ar
          unset CC
          unset CXX


          export CMAKE_TOOLCHAIN_FILE=${ndkPath}/build/cmake/android.toolchain.cmake
          export CMAKE_GENERATOR=Ninja
          export CMAKE_C_COMPILER=${toolchainBinsPath}/aarch64-linux-android24-clang 
          export CMAKE_CXX_COMPILER=${toolchainBinsPath}/aarch64-linux-android24-clang++
          export CC_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang 
          export CXX_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang++ 
          export AWS_LC_SYS_CXX_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang++ 
          export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=${toolchainBinsPath}/aarch64-linux-android24-clang 
          export CFLAGS_AARCH64_LINUX_ANDROID="--target=aarch64-linux-android --sysroot=${prebuiltPath}/sysroot" 
          export CXXFLAGS_AARCH64_LINUX_ANDROID="--target=aarch64-linux-android"
          export BINDGEN_EXTRA_CLANG_ARGS_AARCH64_LINUX_ANDROID="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/aarch64-linux-android" 

          export CC_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang 
          export CXX_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang++ 
          export AWS_LC_SYS_CXX_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang++ 
          export CARGO_TARGET_I686_LINUX_ANDROID_LINKER=${toolchainBinsPath}/i686-linux-android24-clang 
          export CFLAGS_I686_LINUX_ANDROID="--target=i686-linux-android --sysroot=${prebuiltPath}/sysroot" 
          export CXXFLAGS_I686_LINUX_ANDROID="--target=i686-linux-android"
          export BINDGEN_EXTRA_CLANG_ARGS_I686_LINUX_ANDROID="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/i686-linux-android"

          export CC_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang 
          export CXX_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang++ 
          export AWS_LC_SYS_CXX_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang++ 
          export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=${toolchainBinsPath}/x86_64-linux-android24-clang 
          export CFLAGS_X86_64_LINUX_ANDROID="--target=x86_64-linux-android --sysroot=${prebuiltPath}/sysroot" 
          export CXXFLAGS_X86_64_LINUX_ANDROID="--target=x86_64-linux-android"
          export BINDGEN_EXTRA_CLANG_ARGS_X86_64_LINUX_ANDROID="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/x86_64-linux-android" 

          export CC_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang 
          export CXX_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang++ 
          export AWS_LC_SYS_CXX_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang++ 
          export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=${toolchainBinsPath}/armv7a-linux-androideabi24-clang 
          export CFLAGS_ARMV7_LINUX_ANDROID="--target=armv7-linux-androideabi --sysroot=${prebuiltPath}/sysroot" 
          export CXXFLAGS_ARMV7_LINUX_ANDROID="--target=armv7-linux-androideabi" 
          export BINDGEN_EXTRA_CLANG_ARGS_ARMV7_LINUX_ANDROIDEABI="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/arm-linux-androideabi" 
        '';
      };

      packages.androidTauriRust = let
        rust = inputs.holonix.packages.${system}.rust.override {
          extensions = [ "rust-src" "rustfmt" ];
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

      in if pkgs.stdenv.isLinux then linuxCargo else rust;

      devShells.holochainTauriAndroidDev = pkgs.mkShell {
        inputsFrom = [
          self'.devShells.tauriDev
          devShells.androidDev
          inputs'.holochain-nix-builders.devShells.holochainDev
        ];
        packages = [ packages.androidTauriRust ];
        buildInputs =
          inputs.holochain-nix-builders.outputs.dependencies.${system}.holochain.buildInputs
          ++ (with pkgs; [ glibc_multi rust-bindgen ninja cmake ]);

        shellHook = ''
          export PS1='\[\033[1;34m\][p2p-shipyard-android:\w]\$\[\033[0m\] '
        '';
      };
    };
}
