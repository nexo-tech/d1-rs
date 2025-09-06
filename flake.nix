{
  description = "Rust development environment";

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
            "llvm-tools-preview"
          ];
          targets = [ 
            "wasm32-unknown-unknown"
            "aarch64-unknown-linux-musl"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            
            # Build essentials
            pkg-config
            openssl
            
            # Development tools
            cargo-watch
            cargo-edit
            cargo-audit
            cargo-outdated
            cargo-cross
            cargo-make
            cargo-nextest
            cargo-llvm-cov
            cargo-expand
            cargo-bloat
            cargo-udeps
            cargo-machete
            cargo-deny
            bacon
            just
            tokei
            hyperfine
            
            # Debugging tools
            gdb
            lldb
            valgrind
            
            # Additional libraries commonly needed
            zlib
            
            # Wasm tools (optional)
            wasm-pack
            wasmtime
            wasmer
            
            # Documentation
            mdbook
            mdbook-mermaid
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.darwin.apple_sdk.frameworks.CoreServices
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            pkgs.libiconv
          ];
          
          shellHook = ''
            echo "Rust development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "Available tools:"
            echo "  - rust-analyzer (LSP)"
            echo "  - clippy (linter)"
            echo "  - rustfmt (formatter)"
            echo "  - cargo-watch (file watcher)"
            echo "  - cargo-edit (add/rm/upgrade dependencies)"
            echo "  - cargo-audit (security audits)"
            echo "  - cargo-outdated (check for outdated deps)"
            echo "  - cargo-nextest (next-generation test runner)"
            echo "  - cargo-llvm-cov (code coverage)"
            echo "  - cargo-expand (macro expansion)"
            echo "  - bacon (background compiler)"
            echo "  - just (command runner)"
            echo ""
            echo "Run 'nix develop' to enter the shell"
          '';
          
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
        };
      });
}