---
title: "Install"
description: "Install the triplify command with uv, pipx, pip or Cargo on Linux, macOS or Windows."
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

## Check the installation

```sh
triplify --version
triplify --help
```

## Run Triplify

```sh
triplify ./data -o raw.nq.gz
```

See [Usage](usage.md) for input formats and command-line options.
For source builds and local wheel packaging, see [Contributing](contributing.md#build-from-source).
