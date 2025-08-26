{ ... }:

{
  perSystem = { inputs', lib, pkgs, self', ... }: {
    packages.tauri-cli = pkgs.rustPlatform.buildRustPackage rec {
      pname = "tauri-cli";
      version = "2.0.0";

      src = pkgs.fetchCrate {
        inherit pname version;
        hash = "sha256-qVFHfKvLQmAc7CDUDFXQbn7zBAs/lups5c17MsG/KoU=";
      };

      cargoHash = "sha256-Se9U7ZNHcMiS/Rr+/9+XAq7c6x1U44yhtg/huCVSt6o=";
      cargoDepsName = pname;
    };
  };
}
