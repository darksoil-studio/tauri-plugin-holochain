{
  description = "Template for Holochain app development";

  inputs = {
    holonix.url = "github:holochain/holonix/main-0.6";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    playground.url = "github:darksoil-studio/holochain-playground/main-0.6";
    tauri-plugin-holochain.url = "path:../..";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, ... }: rec {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.tauri-plugin-holochain.devShells.holochainTauriDev
            inputs'.holonix.devShells.default
          ];
          packages = [
            # inputs'.tauri-plugin-holochain.packages.hc-pilot
            inputs'.playground.packages.hc-playground
            pkgs.mprocs
          ];
          shellHook = ''
            # Unset redundant _FOR_BUILD and _FOR_TARGET env vars to prevent
            # "Argument list too long" errors when linking. The combined
            # inputsFrom shells create env vars that exceed ARG_MAX (2MB).
            unset NIX_CFLAGS_COMPILE_FOR_BUILD NIX_LDFLAGS_FOR_BUILD
            unset NIX_CFLAGS_COMPILE_FOR_TARGET NIX_LDFLAGS_FOR_TARGET
          '';
        };
        devShells.androidDev = pkgs.mkShell {
          inputsFrom = [
            inputs'.tauri-plugin-holochain.devShells.holochainTauriAndroidDev
            devShells.default
          ];
        };
      };
    };
}
