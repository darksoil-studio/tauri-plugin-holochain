{ inputs, ... }:

{
  perSystem = { inputs', lib, self', system, ... }:

    let pkgs = inputs.gonixpkgs.outputs.legacyPackages.${system};
    in rec {

      packages.custom-go-wrapper = let
        go = packages.custom-go-compiler;
        # there is interference only in this specific case, we assemble a go derivationt that not propagate anything but still has everything available required for our specific use-case
        #
        # the wrapper inherits preconfigured environment variables from the
        # derivation that depends on the propagating go
      in if pkgs.stdenv.isDarwin && pkgs.system == "x86_64-darwin" then
        pkgs.darwin.apple_sdk_11_0.stdenv.mkDerivation {
          name = "go";

          nativeBuildInputs = [ pkgs.makeBinaryWrapper go ];

          dontBuild = true;
          dontUnpack = true;

          installPhase = ''
            makeWrapper ${pkgs.go}/bin/go $out/bin/go \
              ${
                builtins.concatStringsSep " "
                (builtins.map (var: ''--set ${var} "''$${var}"'') [
                  "NIX_BINTOOLS_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                  "NIX_LDFLAGS"
                  "NIX_CFLAGS_COMPILE_FOR_BUILD"
                  "NIX_CFLAGS_COMPILE"

                  # confirmed needed above here

                  # unsure between here
                  # and here

                  # confirmed unneeded below here

                  # "NIX_CC"
                  # "NIX_CC_FOR_BUILD"
                  # "NIX_LDFLAGS_FOR_BUILD"
                  # "NIX_BINTOOLS"
                  # "NIX_CC_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                  # "NIX_CC_WRAPPER_TARGET_BUILD_x86_64_apple_darwin"
                  # "NIX_ENFORCE_NO_NATIVE"
                  # "NIX_DONT_SET_RPATH"
                  # "NIX_BINTOOLS_FOR_BUILD"
                  # "NIX_DONT_SET_RPATH_FOR_BUILD"
                  # "NIX_NO_SELF_RPATH"
                  # "NIX_IGNORE_LD_THROUGH_GCC"
                  # "NIX_PKG_CONFIG_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                  # "NIX_COREFOUNDATION_RPATH"
                  # "NIX_BINTOOLS_WRAPPER_TARGET_BUILD_x86_64_apple_darwin"
                ])
              }
          '';
        }
      else
        go;

      packages.custom-go-compiler = let
        go = lib.overrideDerivation pkgs.go_1_21 (attrs: rec {
          version = "1.21";
          name = "custom-go-${version}-dev";
          src = let
            gitSrc = pkgs.fetchgit {
              url = "https://github.com/wlynxg/go";
              rev = "bff8d409ebfb8d4c8488325f13cb212b07cf6bb4";
              sha256 = "i5MnEkFSEhy+D4C+Syyc0Xkch248VD75ccvQlsMB/6U=";
            };
            finalGo = pkgs.runCommandNoCC "custom-go" { } ''
              mkdir $out
              cd ${gitSrc}
              cp -R . $out
              ls $out

              echo "go${version}" > $out/VERSION
            '';
          in finalGo;
          buildInputs = with pkgs; [ pcre git ];
          nativeBuildInputs = with pkgs; [ pcre git ];
        });

      in go;
    };
}
