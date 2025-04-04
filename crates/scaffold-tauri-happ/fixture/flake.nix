{
  description = "Template for Holochain app development";

  inputs = {
    holonix.url = "github:holochain/holonix";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, ... }: {
        devShells.default =
          pkgs.mkShell { inputsFrom = [ inputs'.holonix.devShells.default ]; };
      };
    };
}
