# Gemini Code Understanding

## Project Overview

This project is a type-safe Object-Relational Mapper (ORM) for Cloudflare D1, written in Rust. It provides a dual backend system, allowing it to work with both Cloudflare D1 (via WASM) for production and native SQLite for testing purposes. The ORM is designed to be type-safe, utilizing derive macros for compile-time query validation. It also includes automatic conversion between SQLite's integer representation of booleans and Rust's `bool` type.

The project is structured as a Rust workspace with the main ORM crate (`d1-rs`) and a derive macro crate (`d1-rs-derive`).

## Building and Running

### Dependencies

- **Rust:** The project is built with Cargo.
- **Just:** The `justfile` provides a simple command for running tests.
- **SQLite:** Required for running tests locally.

### Commands

- **Running Tests:**
  - The recommended way to run tests is using the `just` command:
    ```bash
    just test
    ```
  - Alternatively, you can use `cargo` directly:
    ```bash
    cargo test --features test-utils --no-default-features
    ```
  - The `run_tests.sh` script provides a more detailed test execution, including environment setup for macOS.

- **Building:**
  - To build the project, use the standard `cargo build` command.
  - To build for the `wasm32` target, you will need to specify the target architecture:
    ```bash
    cargo build --target wasm32-unknown-unknown
    ```

## Development Conventions

- **Testing:** The project has a comprehensive test suite in the `tests` directory. Tests are written using the standard Rust testing framework and rely on `rusqlite` for in-memory database testing.
- **Conditional Compilation:** The ORM uses conditional compilation (`#[cfg(target_arch = "wasm32")]`) to separate the Cloudflare D1 and SQLite backends. This ensures that there is no runtime overhead for the unused backend.
- **Macros:** The `d1-rs-derive` crate provides procedural macros for deriving the `Entity` trait, which is the core of the ORM's type safety.
- **Error Handling:** The library uses a custom `D1RsError` enum for error handling, which covers database errors, not found errors, validation errors, and serialization errors.
