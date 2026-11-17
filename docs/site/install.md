---
title: "Install"
description: "Install the triplify command from source with Cargo today, or from PyPI and crates.io once the first release is published."
permalink: /install/
nav_order: 2
grapharch:
  subject: urn:triplify:website:page:install
---

# Install

Triplify is a single native executable, `triplify`, written in Rust. The
package is named `triplify` on both PyPI and crates.io.

## What works today

No release has been published yet, so `uv tool install triplify`,
`pipx install triplify`, `pip install triplify` and `cargo install triplify`
do not work yet. Until the first release, install from source.

### From source with Cargo

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

### As a Python wheel built locally

Triplify can also be packaged as a [maturin](https://www.maturin.rs/) `bin`
wheel, the same as GraphArch's. The wheel contains the native executable, so it
needs no Python dependencies and downloads nothing at runtime. Build one from a
checkout and install it into an isolated tool environment:

```sh
uvx maturin build --release --locked --out dist
uv tool install dist/triplify-*.whl
```

## Once the first release is published

The release workflow publishes to PyPI and crates.io under the name
`triplify`. After that, any of these installs the `triplify` command:

```sh
uv tool install triplify
pipx install triplify
pip install triplify
cargo install triplify
```

The wheels cover Linux (glibc 2.28 or later), macOS 11 or later and Windows,
on x86-64 and ARM64.

## Check the installation

```sh
triplify --version
triplify --help
```

The [Usage](usage.md) page explains every option.
