{ inputs, ... }:

{
  perSystem = { inputs', self', pkgs, system, lib, ... }: {

    packages.scaffold-holochain-runtime = let
      craneLib = inputs.crane.mkLib pkgs;

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;

      commonArgs = {
        src = (inputs.scaffolding.outputs.lib.cleanScaffoldingSource {
          inherit lib;
        }) (craneLib.path ../../.);
        doCheck = false;
        buildInputs = self'.dependencies.tauriHapp.buildInputs;
        nativeBuildInputs = self'.dependencies.tauriHapp.nativeBuildInputs;
        cargoExtraArgs = "--locked --package scaffold-holochain-runtime";
      };
    in craneLib.buildPackage (commonArgs // {
      pname = crate;
      version = cargoToml.package.version;
    });

  };
}
