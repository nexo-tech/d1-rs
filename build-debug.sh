#!/bin/bash

echo "🔨 Building with debug symbols and source maps..."

# Build with debug info
export RUSTFLAGS="-C debuginfo=2 -C opt-level=0"
export WASM_BINDGEN_DEBUG=1

# Use wasm-pack for better source map support
if command -v wasm-pack &> /dev/null; then
    echo "Using wasm-pack for better debugging..."
    wasm-pack build --dev --target web --out-dir build/debug
else
    echo "Building with worker-build (limited debugging)..."
    # Build with worker-build but keep debug symbols
    worker-build --dev
    
    # Try to generate source maps with wasm-bindgen if available
    if command -v wasm-bindgen &> /dev/null; then
        echo "Generating source maps..."
        wasm-bindgen target/wasm32-unknown-unknown/debug/*.wasm \
            --out-dir build/worker \
            --target web \
            --debug \
            --keep-debug \
            --no-typescript
    fi
fi

echo "✅ Debug build complete with source maps!"