---
title: "Install"
description: "Install the triplify command with uv, pipx, pip or Cargo, or build it from source."
permalink: /install/
nav_order: 2
grapharch:
  subject: urn:triplify:website:page:install
---

# Install

Triplify is a single native executable, `triplify`, written in Rust. The
package is named `triplify` on both PyPI and crates.io.

## Install from PyPI

Install the native executable in an isolated tool environment with uv:

```sh
uv tool install triplify
```

You can also use pipx or pip:

```sh
pipx install triplify
pip install triplify
```

The wheels contain the native executable, need no Python dependencies and
download nothing at runtime. They cover Linux (glibc 2.28 or later), macOS 11
or later and Windows, on x86-64 and ARM64.

## Install with Cargo

```sh
cargo install triplify
```

## Build from source

The repository pins a nightly Rust toolchain in `rust-toolchain.toml`, which
`rustup` selects automatically. Install the latest `main` with:

```sh
cargo install --locked --git https://github.com/EKGF/triplify
```

Or build a checkout:

```sh
git clone https://github.com/EKGF/triplify
cd triplify
cargo build --release --locked
./target/release/triplify --version
```

### Build a Python wheel

Triplify can also be packaged as a [maturin](https://www.maturin.rs/) `bin`
wheel. The wheel contains the native executable, so it needs no Python
dependencies and downloads nothing at runtime. Build one from a checkout and
install it into an isolated tool environment:

```sh
uvx maturin build --release --locked --out dist
uv tool install dist/triplify-*.whl
```

## Check the installation

```sh
triplify --version
triplify --help
```

The [Usage](usage.md) page explains every option.
