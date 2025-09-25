{
  description = "d1-rs multi-database development environment";

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
        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            cargo-nextest
            
            # Database engines
            sqlite
            postgresql_15
            mysql80
            
            # Database tools
            pgcli
            mycli
            sqlite-utils
            
            # Development tools
            docker
            docker-compose
            just
            watchexec
            
            # System dependencies
            pkg-config
            openssl
            zlib
            bzip2
            worker-build
            nodejs_22
            libiconv
            
            # PostgreSQL development libraries
            postgresql.dev
            
            # MySQL development libraries  
            mysql80.dev
            libmysqlclient
            
            # Additional utilities
            jq
            curl
            git
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
            export PKG_CONFIG_PATH="${pkgs.sqlite.dev}/lib/pkgconfig:${pkgs.postgresql.dev}/lib/pkgconfig:${pkgs.mysql80.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            
            # Claude Code timeout settings
            export BASH_DEFAULT_TIMEOUT_MS="1800000"  # 30 minutes
            export BASH_MAX_TIMEOUT_MS="7200000"      # 2 hours
            
            # Set up PostgreSQL
            export PGDATA=$PWD/postgres_data
            export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5434/d1rs_test"
            export POSTGRES_DEV_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_dev"
            
            # Set up MySQL
            export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3308/d1rs_test"
            export MYSQL_DEV_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_dev"
            
            # SQLite (for testing)
            export SQLITE_TEST_URL="sqlite::memory:"
            export SQLITE_DEV_URL="./dev.db"
            
            # Testing Configuration
            export TEST_TIMEOUT=300
            export TEST_PARALLEL_JOBS=4
            
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
            
            # Development shortcuts
            alias db:setup="docker-compose -f docker-compose.dev.yml up -d"
            alias db:stop="docker-compose -f docker-compose.dev.yml down"
            alias db:reset="docker-compose -f docker-compose.dev.yml down -v && docker-compose -f docker-compose.dev.yml up -d"
            alias test:all="just test"
            alias test:postgres="POSTGRES_TEST_URL=$POSTGRES_TEST_URL just test-filter postgres"
            alias test:mysql="MYSQL_TEST_URL=$MYSQL_TEST_URL just test-filter mysql"
            alias test:sqlite="just test-filter sqlite"
            
            echo "🚀 d1-rs multi-database development environment loaded!"
            echo "======================================================="
            echo "📦 Available databases: PostgreSQL, MySQL, SQLite"
            echo "🧰 Development tools: Docker, pgcli, mycli, sqlite-utils"
            echo ""
            echo "🐘 PostgreSQL URLs:"
            echo "  DEV:  $POSTGRES_DEV_URL"
            echo "  TEST: $POSTGRES_TEST_URL"
            echo ""
            echo "🐬 MySQL URLs:"
            echo "  DEV:  $MYSQL_DEV_URL"
            echo "  TEST: $MYSQL_TEST_URL"
            echo ""
            echo "🗄️  SQLite URLs:"
            echo "  DEV:  $SQLITE_DEV_URL"
            echo "  TEST: $SQLITE_TEST_URL"
            echo ""
            echo "🧪 Database Management:"
            echo "  db:setup    - Start development databases"
            echo "  db:stop     - Stop development databases"
            echo "  db:reset    - Reset databases (clean slate)"
            echo ""
            echo "🏃 Testing Commands:"
            echo "  test:all       - Run ALL tests (all databases)"
            echo "  test:postgres  - Run PostgreSQL-specific tests"
            echo "  test:mysql     - Run MySQL-specific tests"
            echo "  test:sqlite    - Run SQLite-specific tests"
            echo ""
            echo "🚀 Standard Testing (nextest):"
            echo "  just test              - Run ALL tests (fast parallel execution)"
            echo "  just test-one <name>   - Run specific test by name"
            echo "  just test-filter <pat> - Run tests matching pattern"
            echo "  just test-debug <name> - Debug single test with full output"
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

