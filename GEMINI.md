# Gemini Code Understanding

## Project Overview

This project, "HexScope," is a desktop application for viewing and editing files in hexadecimal and ASCII formats. It is written in Rust and uses the `egui` GUI library via the `eframe` framework. A key feature is its side-by-side diffing capability, allowing users to compare two files and visually identify differing bytes.

**Key Technologies:**

*   **Language:** Rust (2021 Edition)
*   **GUI Framework:** `egui`
*   **GUI Backend:** `eframe`
*   **Dependencies:**
    *   `rfd`: For native file open/save dialogs.

**Architecture:**

The application follows a simple, stateful GUI architecture:

*   **`main.rs`**: The application entry point. It initializes `eframe` and creates the main application window.
*   **`hex_app.rs`**: This is the core of the application. The `HexApp` struct holds all application state, including the data for the two files being compared, UI settings, and user interaction state (like cursor position and edit mode). It implements the `eframe::App` trait, which defines the main update loop where all UI rendering and event handling occurs.
*   **`file_buffer.rs`**: This module provides the `FileBuffer` struct, a simple container for the byte data of a file, its path, and a `dirty` flag to track whether it has unsaved changes.

## Building and Running

Standard Cargo commands are used for building and running the project.

**Build:**
To build the project in release mode:
```bash
cargo build --release
```
The executable will be located at `target/release/hexscope`.

**Run:**
To build and run the project in debug mode:
```bash
cargo run
```

**Testing:**
There are currently no tests in the project.

## Development Conventions

*   **Code Style:** The code generally follows standard Rust conventions. It is formatted with `rustfmt`.
*   **Error Handling:** Error handling is minimal. File I/O errors are printed to the console (`eprintln!`).
*   **Mutability:** The application state is managed within a single mutable `HexApp` struct, which is passed to all rendering and logic functions.
*   **No Unsafe Code:** The project does not use any `unsafe` code blocks.
*   **Modules:** The code is organized into three main modules: `main`, `hex_app`, and `file_buffer`.
*   **Edition:** The `Cargo.toml` file specifies the Rust 2024 edition, but the `Cargo.lock` file indicates that the project was created with the 2021 edition. This should be corrected to 2021 in `Cargo.toml`.
