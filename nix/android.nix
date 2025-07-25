{ inputs, ... }:

{
  perSystem = { inputs', lib, pkgs, self', system, ... }: rec {
    packages.android-sdk = let
      pkgs = import inputs.nixpkgs {
        inherit system;
        config.allowUnfree = true;
        config.android_sdk.accept_license = true;
      };
    in (pkgs.androidenv.composeAndroidPackages {
      platformVersions = [ "30" "34" "35" "36" ];
      buildToolsVersions = [ "30.0.3" "34.0.0" "35.0.0" "36.0.0" ];
      systemImageTypes = [ "google_apis_playstore" ];
      abiVersions = [ "armeabi-v7a" "arm64-v8a" "x86" "x86_64" ];
      includeNDK = true;
      # ndkVersion = "25.2.9519653";

      ndkVersion = "28.0.13004108";

      # ndkVersion = "27.0.12077973";
      # ndkVersion = "28.1.13356709";

      # includeExtras = [ "extras" "google" "auto" ];
    }).androidsdk;

    devShells.androidDev = pkgs.mkShell {
      packages = [ packages.android-sdk pkgs.gradle pkgs.jdk17 pkgs.aapt ];

      shellHook = let
        sdk = packages.android-sdk;
        sdkPath = "${sdk}/libexec/android-sdk";
        ndkPath = "${sdkPath}/ndk-bundle";
        platform = if pkgs.stdenv.isLinux then "linux" else "darwin";
        toolchainSystem = "${platform}-x86_64";
        prebuiltPath = "${ndkPath}/toolchains/llvm/prebuilt/${toolchainSystem}";
        toolchainBinsPath = "${prebuiltPath}/bin";
        clangVersion = "19";

      in ''
        export GRADLE_OPTS="-Dorg.gradle.project.android.aapt2FromMavenOverride=${sdkPath}/build-tools/34.0.0/aapt2"
        export GRADLE_HOME=${pkgs.gradle}

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
        export CLANG_PATH=${toolchainBinsPath}/clang 
        export LIBCLANG_PATH=${prebuiltPath}/lib

        export CC_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang 
        export CXX_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang++ 
        export AWS_LC_SYS_CXX_aarch64_linux_android=${toolchainBinsPath}/aarch64-linux-android24-clang++ 
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=${toolchainBinsPath}/aarch64-linux-android24-clang 
        export DISABLED_CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-L${prebuiltPath}/lib/clang/${clangVersion}/lib/${platform} -lstatic=clang_rt.builtins-aarch64-android"
        export CFLAGS_aarch64_linux_android="--target=aarch64-linux-android24 --sysroot=${prebuiltPath}/sysroot" 
        export CXXFLAGS_aarch64_linux_android="--target=aarch64-linux-android24" 
        export BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/aarch64-linux-android" 

        export CC_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang 
        export CXX_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang++ 
        export AWS_LC_SYS_CXX_i686_linux_android=${toolchainBinsPath}/i686-linux-android24-clang++ 
        export CARGO_TARGET_I686_LINUX_ANDROID_LINKER=${toolchainBinsPath}/i686-linux-android24-clang 
        export DISABLED_CARGO_TARGET_I686_LINUX_ANDROID_RUSTFLAGS="-L${prebuiltPath}/lib/clang/${clangVersion}/lib/${platform} -lstatic=clang_rt.builtins-i686-android"
        export CFLAGS_i686_linux_android="--target=i686-linux-android24 --sysroot=${prebuiltPath}/sysroot" 
        export CXXFLAGS_i686_linux_android="--target=i686-linux-android24" 
        export BINDGEN_EXTRA_CLANG_ARGS_i686_linux_android="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/i686-linux-android"

        export CC_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang 
        export CXX_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang++ 
        export AWS_LC_SYS_CXX_x86_64_linux_android=${toolchainBinsPath}/x86_64-linux-android24-clang++ 
        export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=${toolchainBinsPath}/x86_64-linux-android24-clang 
        export DISABLED_CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="-L${prebuiltPath}/lib/clang/${clangVersion}/lib/${platform} -lstatic=clang_rt.builtins-x86_64-android"
        export CFLAGS_x86_64_linux_android="--target=x86_64-linux-android24 --sysroot=${prebuiltPath}/sysroot" 
        export CXXFLAGS_x86_64_linux_android="--target=x86_64-linux-android24" 
        export BINDGEN_EXTRA_CLANG_ARGS_x86_64_linux_android="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/x86_64-linux-android" 

        export CC_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang 
        export CXX_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang++ 
        export AWS_LC_SYS_CXX_armv7_linux_androideabi=${toolchainBinsPath}/armv7a-linux-androideabi24-clang++ 
        export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=${toolchainBinsPath}/armv7a-linux-androideabi24-clang 
        export DISABLED_CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS="-L${prebuiltPath}/lib/clang/${clangVersion}/lib/${platform} -lstatic=clang_rt.builtins-arm-android"
        export CFLAGS_armv7_linux_androideabi="--target=armv7-linux-androideabi24 --sysroot=${prebuiltPath}/sysroot" 
        export CXXFLAGS_armv7_linux_androideabi="--target=armv7-linux-androideabi24" 
        export BINDGEN_EXTRA_CLANG_ARGS_armv7_linux_androideabi="--sysroot=${prebuiltPath}/sysroot -I${prebuiltPath}/sysroot/usr/include/arm-linux-androideabi"
      '';
    };

    packages.androidTauriRust = let
      rust = self'.packages.rust.override {
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

    in rust;

    devShells.holochainTauriAndroidDev = pkgs.mkShell {
      inputsFrom = [
        self'.devShells.tauriDev
        devShells.androidDev
        inputs'.holochain-nix-builders.devShells.holochainDev
      ];
      packages = [ packages.androidTauriRust ];
      buildInputs =
        inputs.holochain-nix-builders.outputs.dependencies.${system}.holochain.buildInputs
        ++ (with pkgs; [ rust-bindgen ninja cmake openssl ])
        ++ (lib.optionals pkgs.stdenv.isLinux [ pkgs.glibc_multi ]);

      shellHook = ''
        export PS1='\[\033[1;34m\][tauri-plugin-holochain-android:\w]\$\[\033[0m\] '
      '';
    };
  };
}
