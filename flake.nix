{
  description = "Minimal Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
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
        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            just
            nodejs_22
            bzip2
            worker-build
            # Dependencies for ORM testing
            sqlite
            libiconv
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS-specific dependencies
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          shellHook = ''
            export PATH="$HOME/.cargo/bin:$PATH"
            export LDFLAGS="-L${pkgs.bzip2}/lib -L${pkgs.sqlite.out}/lib -L${pkgs.libiconv}/lib $LDFLAGS"
            export CPPFLAGS="-I${pkgs.bzip2}/include -I${pkgs.sqlite.dev}/include -I${pkgs.libiconv}/include $CPPFLAGS"
            export PKG_CONFIG_PATH="${pkgs.sqlite.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            
            # macOS framework paths
            ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
              export LDFLAGS="-F${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks $LDFLAGS"
              export NIX_CFLAGS_COMPILE="-F${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks $NIX_CFLAGS_COMPILE"
              export RUSTFLAGS="-L framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -L framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -L framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks $RUSTFLAGS"
            ''}
            
            # Generate dynamic .cargo/config.toml with correct framework paths
            ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
              mkdir -p .cargo
              cat > .cargo/config.toml << EOF
[target.aarch64-apple-darwin]
rustflags = [
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks", 
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks",
  "-L", "${pkgs.libiconv}/lib",
]

[target.x86_64-apple-darwin]  
rustflags = [
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks",
  "-L", "${pkgs.libiconv}/lib",
]
EOF
            ''}
            
            echo "🦀 Cloudflare Worker Rust Development Environment"
            echo "================================================"
            echo "Rust version: $(rustc --version)"
            echo "Tools: rust-analyzer, clippy, rustfmt, just"
            echo ""
            echo "Available commands:"
            echo "  just dev               - Start local development"
            echo "  just build             - Build for production"  
            echo "  just deploy            - Deploy to Cloudflare"
            echo "  just test-orm          - Run ALL ORM tests (comprehensive)"
            echo "  just test-orm-basic    - Run basic ORM tests only"
          '';
          
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
          
          # Expose framework paths to Rust linker  
          RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.isDarwin 
            "-L framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -L framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -L framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks";
            
          # Additional linker flags for macOS frameworks
          NIX_LDFLAGS = pkgs.lib.optionalString pkgs.stdenv.isDarwin
            "-F${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks -F${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks -framework CoreFoundation -framework Security -framework SystemConfiguration";
        };
      });
}

