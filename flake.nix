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
            # Fast test runner
            cargo-nextest
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            # Linux-specific dependencies for faster compilation
            mold
            clang
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
            
            # Generate dynamic .cargo/config.toml with platform-specific optimizations
            mkdir -p .cargo
            cat > .cargo/config.toml << EOF
${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
# Linux targets with mold linker for faster compilation
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"  
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
''}
${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
# macOS targets with optimized linking and framework paths
[target.aarch64-apple-darwin]
rustflags = [
  "-C", "link-arg=-Wl,-dead_strip",
  "-C", "link-arg=-Wl,-no_compact_unwind",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks", 
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks",
  "-L", "${pkgs.libiconv}/lib",
]

[target.x86_64-apple-darwin]  
rustflags = [
  "-C", "link-arg=-Wl,-dead_strip",
  "-C", "link-arg=-Wl,-no_compact_unwind",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks",
  "-L", "framework=${pkgs.darwin.apple_sdk.frameworks.SystemConfiguration}/Library/Frameworks",
  "-L", "${pkgs.libiconv}/lib",
]
''}

# Global compilation optimizations for faster builds
[build]
# jobs setting removed - cargo auto-detects available cores by default

[profile.dev]
opt-level = 1        # Slight optimization for faster debug builds
debug = 1            # Reduced debug info for faster compilation
incremental = true   # Enable incremental compilation
EOF
            
            echo "🦀 Cloudflare Worker Rust Development Environment"
            echo "================================================"
            echo "Rust version: $(rustc --version)"
            echo "Tools: rust-analyzer, clippy, rustfmt, just"
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              echo "Linker: mold (fast linking enabled)"
            ''}
            ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
              echo "Linking: Apple ld64 with optimizations enabled"
            ''}
            echo ""
            echo "Available commands:"
            echo "  just dev               - Start local development"
            echo "  just build             - Build for production"  
            echo "  just deploy            - Deploy to Cloudflare"
            echo ""
            echo "🚀 Fast Testing (nextest):"
            echo "  just test              - Run ALL tests (fast parallel execution)"
            echo "  just test-one <name>   - Run specific test by name"
            echo "  just test-filter <pat> - Run tests matching pattern"
            echo "  just test-quick        - Quick smoke tests only"
            echo "  just test-fast         - Ultra-fast with aggressive timeouts"
            echo "  just test-failed       - Re-run only failed tests"
            echo "  just test-debug <name> - Debug single test with full output"
            echo "  just test-clean        - Clean locks if tests hang"
            echo "  just test-legacy       - Use traditional cargo test (slower)"
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

