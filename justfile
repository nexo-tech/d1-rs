# Run ORM tests (all tests - no filtering)
test:
    @echo "🧪 Running ALL D1 ORM tests..."
    cargo test --target $(rustc -vV | sed -n 's|host: ||p')

