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

      cargoHash = "sha256-sXkJGNeeC6Osy++eNvWSX1TGD+aTaQ4n9FHMSed2DFI=";
      cargoDepsName = pname;
    };
  };
}
