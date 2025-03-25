# Copyright 2022-2024 TII (SSRC) and the Ghaf contributors
# SPDX-License-Identifier: Apache-2.0
{
  description = "Wireguard development dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    rust-overlay,
    advisory-db,
    ...
  }:
    flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux"] (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        # FIXME: It breaks the check : "nix flake check --no-build -L --no-eval-cache --store $TMPSTORE"
        #src = craneLib.cleanCargoSource ./.;
        src = ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [pkg-config wrapGAppsHook];

          buildInputs = with pkgs; [
            wireguard-tools
            glib
            gtk4
            polkit
            gsettings-desktop-schemas
          ];
        };

        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
            doNotPostBuildInstallCargoBinaries = true;

            doCheck = false;
            cargoVendorDir = craneLib.vendorMultipleCargoDeps {
              inherit (craneLib.findCargoFiles src) cargoConfigs;
              cargoLockList = [./Cargo.lock];
            };
          };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        fileSetForCrate = crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./src
              crate
            ];
          };

        # Sequential flake checking can be utilized for CI/CD purposes.
        # Run squence cmd: 'nix flake check'
        # 1. Check formatting
        wireguardGuiPackage-cargoFmt = craneLib.cargoFmt (
          individualCrateArgs // {inherit src cargoArtifacts;}
        );

        #  2. Run clippy (and deny all warnings) on the crate source.
        wireguardGuiPackage-cargoClippy = craneLib.cargoClippy (
          individualCrateArgs
          // {
            # Again we apply some extra arguments only to this derivation
            # and not every where else. In this case we add some clippy flags
            cargoArtifacts = wireguardGuiPackage-cargoFmt;
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            preBuild = ''
              cargo build --release
            '';
            cargoClippyExtraArgs = "-- --deny warnings";
          }
        );

        # 3. we want to run the tests and collect code-coverage, _but only if
        # the clippy checks pass_ so we do not waste any extra cycles.
        wireguardGuiPackage-cargoTarpaulin = craneLib.cargoTarpaulin (
          individualCrateArgs // {cargoArtifacts = wireguardGuiPackage-cargoClippy;}
        );

        # 4. cargo-audit
        wireguardGuiPackage-cargoAudit = craneLib.cargoAudit (
          individualCrateArgs
          // {
            inherit advisory-db;
            cargoArtifacts = wireguardGuiPackage-cargoTarpaulin;
          }
        );

        mkwireguardGuiPackage = buildType:
          craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "wireguard-gui";
              cargoExtraArgs = "";
              src = fileSetForCrate ./.;
              #CARGO_BUILD_RUSTFLAGS = "-C link-arg=-lasan -Zproc-macro-backtrace";
              nativeBuildInputs = with pkgs; [
                openssl
                pkg-config
                eza
                fd
                clang
                cargo-audit
                wrapGAppsHook4
              ];
              buildPhaseCargoCommand = ''
                if [[ "${buildType}" == "release" ]]; then
                     cargo build --release
                  else
                     cargo build
                  fi

              '';

              installPhase = ''
                mkdir -p $out/bin
                install target/${buildType}/wireguard-gui $out/bin/wireguard-gui
              '';
              postFixup = ''
                wrapProgram $out/bin/wireguard-gui \
                  --set LIBGL_ALWAYS_SOFTWARE true \
                  --set G_MESSAGES_DEBUG all
              '';
            }
          );
        # Create packages for different build types
        wireguardGuiRelease = mkwireguardGuiPackage "release";
        wireguardGuiDebug = mkwireguardGuiPackage "debug";
      in
        with pkgs; {
          formatter = pkgs.alejandra;
          packages = {
            inherit wireguardGuiRelease wireguardGuiDebug;
            default = wireguardGuiRelease; # Default to release build
          };
          checks = {
            inherit
              # Build the crate as part of `nix flake check` for convenience
              wireguardGuiRelease
              wireguardGuiPackage-cargoTarpaulin

              ;
          };
          devShells.default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self.checks.${system};
            inherit (commonArgs) buildInputs;
          };
        }
    )
    // {
      overlays.default = final: prev: {
        wireguard-gui = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
