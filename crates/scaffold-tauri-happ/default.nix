{ inputs, self, ... }:

{
  perSystem = { inputs', self', pkgs, system, lib, ... }: {

    packages.scaffold-tauri-happ = let
      craneLib = inputs.crane.mkLib pkgs;

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;

      commonArgs = {
        src = (self.lib.cleanScaffoldingSource { inherit lib; })
          (craneLib.path ../../.);
        doCheck = false;
        buildInputs = self'.dependencies.tauriHapp.buildInputs;
        nativeBuildInputs = self'.dependencies.tauriHapp.nativeBuildInputs;
        cargoExtraArgs = "--locked --package scaffold-tauri-happ";
      };
    in craneLib.buildPackage (commonArgs // {
      pname = crate;
      version = cargoToml.package.version;
    });
  };
}
