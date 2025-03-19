{ craneLib, buildInputs, nativeBuildInputs }:
let
  src = craneLib.cleanCargoSource (craneLib.path ./..);
  commonArgs = {
    inherit src buildInputs nativeBuildInputs;
    CARGO_PROFILE = "release";

    doCheck = false;
    cargoExtraArgs = "--tests -p reference-tauri-happ";
  };
  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    pname = "tauri-happ";
    version = "for-holochain-0.4.x";
  });
in cargoArtifacts
