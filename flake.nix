{
  description = "Minimal Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ 
            "rust-src" 
            "rust-analyzer" 
            "clippy" 
            "rustfmt" 
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            just
            nodejs_22
          ];
          
          shellHook = ''
            export PATH="/home/snowbear/.cargo/bin:$PATH"
            echo "🦀 Cloudflare Worker Rust Development Environment"
            echo "================================================"
            echo "Rust version: $(rustc --version)"
            echo "Wrangler version: $(wrangler --version)"
            echo "Tools: rust-analyzer, clippy, rustfmt, just"
            echo "Target: wasm32-unknown-unknown (for Cloudflare)"
            echo ""
            echo "Available commands:"
            echo "  just         - Show all available commands"
            echo "  just setup   - Initial project setup"
            echo "  just dev     - Start local development"
            echo "  just deploy  - Deploy to Cloudflare"
          '';
          
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
        };
      });
}

