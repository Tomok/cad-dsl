{
  description = "CAD-DSL - Domain Specific language for Constraint-based 2D/3D CAD system";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "llvm-tools-preview" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          clang
          libclang.dev
          cargo-llvm-cov  # Code coverage tool
        ];

        buildInputs = with pkgs; [
          z3  # System Z3 - won't be compiled by cargo!
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          # Environment variables for z3-sys crate
          Z3_SYS_Z3_HEADER = "${pkgs.z3.dev}/include/z3.h";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          shellHook = ''
            echo "CAD-DSL development environment"
            echo "Rust: $(rustc --version)"
            echo "Z3: $(z3 --version)"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "textcad";
          version = "0.1.0";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          Z3_SYS_Z3_HEADER = "${pkgs.z3.dev}/include/z3.h";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };
      }
    );
}
