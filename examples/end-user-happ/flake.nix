{
  description = "Template for Holochain app development";

  inputs = {
    holonix.url = "github:holochain/holonix/main-0.5";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    playground.url = "github:darksoil-studio/holochain-playground/main-0.5";
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
