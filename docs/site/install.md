---
title: "Install"
description: "Build and install the triplifier command from source with Cargo, or build its native Python wheel with maturin."
permalink: /install/
nav_order: 2
grapharch:
  subject: urn:triplifier:website:page:install
---

# Install

Triplifier is a single native executable, `triplifier`, written in Rust.

## From source with Cargo

The repository pins a nightly Rust toolchain in `rust-toolchain.toml`, which
`rustup` selects automatically. Install the latest `main` with:

```sh
cargo install --locked --git https://github.com/EKGF/triplifier
```

Or build a checkout:

```sh
git clone https://github.com/EKGF/triplifier
cd triplifier
cargo build --release --locked
./target/release/triplifier --version
```

## As a Python wheel

Triplifier can also be packaged as a [maturin](https://www.maturin.rs/) `bin`
wheel, the same as GraphArch's. The wheel contains the native executable, so it
needs no Python dependencies and downloads nothing at runtime. Build one from a
checkout and install it into an isolated tool environment:

```sh
uvx maturin build --release --locked --out dist
uv tool install dist/triplifier-*.whl
```

The release workflow builds wheels for Linux (glibc 2.28 or later), macOS 11
or later and Windows, on x86-64 and ARM64.

## Check the installation

```sh
triplifier --version
triplifier --help
```

The [Usage](usage.md) page explains every option.
