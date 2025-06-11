{ inputs, self, ... }:

{
  perSystem = { inputs', self', pkgs, system, lib, ... }: {
    packages.hc-pilot = let
      rust = self'.packages.rust;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust;

      cratePath = ./.;

      cargoToml =
        builtins.fromTOML (builtins.readFile "${cratePath}/Cargo.toml");
      crate = cargoToml.package.name;

      commonArgs = {
        src =
          (self.lib.cleanTauriSource { inherit lib; }) (craneLib.path ../../.);
        doCheck = false;

        buildInputs = self'.dependencies.tauriHapp.buildInputs;
        nativeBuildInputs = self'.dependencies.tauriHapp.nativeBuildInputs;

        # TODO: remove this if possible
        # Without this build fails on MacOs
        postPatch = ''
          mkdir -p "$TMPDIR/nix-vendor"
          cp -Lr "$cargoVendorDir" -T "$TMPDIR/nix-vendor"
          sed -i "s|$cargoVendorDir|$TMPDIR/nix-vendor/|g" "$TMPDIR/nix-vendor/config.toml"
          chmod -R +w "$TMPDIR/nix-vendor"
          cargoVendorDir="$TMPDIR/nix-vendor"
        '';

        # Make sure libdatachannel can find C++ standard libraries from clang.
        LIBCLANG_PATH = "${pkgs.llvmPackages_18.libclang.lib}/lib";
      };
      # cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
      #   pname = crate;
      #   version = cargoToml.package.version;
      # });
      binary = craneLib.buildPackage (commonArgs // {
        pname = crate;
        version = cargoToml.package.version;
        # inherit cargoArtifacts;
      });
    in pkgs.runCommandNoCC crate { buildInputs = [ pkgs.makeWrapper ]; } ''
      mkdir $out
      mkdir $out/bin
      # Because we create this ourself, by creating a wrapper
      makeWrapper ${binary}/bin/hc-pilot $out/bin/hc-pilot \
        --set WEBKIT_DISABLE_DMABUF_RENDERER 1
    '';
  };
}
