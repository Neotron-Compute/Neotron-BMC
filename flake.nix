{
  description = "Neotron BMC";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    let
      # List of systems this flake.nix has been tested to work with
      systems = [ "x86_64-linux" ];
      pkgsForSystem = system: nixpkgs.legacyPackages.${system}.appendOverlays [
        rust-overlay.overlays.default
      ];
    in
    flake-utils.lib.eachSystem systems
      (system:
        let
          pkgs = pkgsForSystem system;
          toolchain = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "thumbv6m-none-eabi" ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          bmc-pico = craneLib.mkCargoDerivation {
            pname = "neotron-bmc-pico";
            version = (craneLib.crateNameFromCargoToml {
              cargoToml = ./neotron-bmc-pico/Cargo.toml;
            }).version;
            cargoArtifacts = null;
            cargoVendorDir = craneLib.vendorCargoDeps { src = ./neotron-bmc-pico; };
            src = with pkgs.lib;
              let keep = suffixes: path: type: any (s: hasSuffix s path) suffixes;
              in
                cleanSourceWith {
                  src = craneLib.path ./.;
                  filter = path: type: any id (map (f: f path type) [
                    craneLib.filterCargoSources
                    (keep [
                      ".md" # neotron-bmc-protocol needs README.md
                      ".x"
                    ])
                  ]);
                };
            buildPhaseCargoCommand = ''
              cd neotron-bmc-pico # because .cargo/config.toml exists here
              cargo build --release --target=thumbv6m-none-eabi
            '';
            nativeBuildInputs = [
              pkgs.makeWrapper
              pkgs.probe-run
              pkgs.flip-link
            ];
            installPhase = ''
              mkdir -p $out/bin
              cp target/thumbv6m-none-eabi/release/neotron-bmc-pico $out/bin/
              makeWrapper ${pkgs.probe-run}/bin/probe-run $out/bin/flash-neotron-bmc-pico \
                --set DEFMT_LOG info \
                --add-flags "--chip STM32F030K6Tx $out/bin/neotron-bmc-pico"
            '';
          };
        in rec
        {
          packages.neotron-bmc-pico = bmc-pico;
          defaultPackage = packages.neotron-bmc-pico;

          defaultApp = apps.neotron-bmc-pico;
          apps.neotron-bmc-pico = flake-utils.lib.mkApp {
            drv = bmc-pico;
            exePath = "/bin/flash-neotron-bmc-pico";
          };

          devShell = pkgs.mkShell {
            nativeBuildInputs = [
              toolchain
              pkgs.probe-run
              pkgs.flip-link
            ];
          };
        }
      );
}
