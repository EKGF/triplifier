---
title: "Usage"
description: "Command-line reference for triplify: input paths, output formats chosen by file name, gzip compression and RDFC-1.0 canonical output."
permalink: /usage/
nav_order: 3
grapharch:
  subject: urn:triplify:website:page:usage
---

# Usage

One command, a list of paths and an optional output file.

## Command-line reference

This is the output of `triplify --help`:

```text
Triplify turns any tabular data dump (CSV, XLSX, ...) into raw RDF using the EKGF Raw Data Ontology, with DCAT, PROV and git provenance, without any mapping

Usage: triplify [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Files or directories to triplify (directories are walked, honouring .gitignore) [default: .]

Options:
  -o, --output <OUTPUT>  Output file: .nq, .nt (graphs merged) or .trig, optionally .gz. Defaults to N-Quads on stdout
      --canonical        Emit RDFC-1.0 canonical N-Quads/N-Triples (buffers the dataset in memory)
  -v, --verbose...       Increase logging verbosity
  -q, --quiet...         Decrease logging verbosity
  -h, --help             Print help
  -V, --version          Print version
```

## Inputs

Give any mix of files and directories; with none, triplify reads the current
directory. Directories are walked in parallel. The walk honours `.gitignore`,
`.git/info/exclude` and your global Git excludes, and skips hidden files and
directories.

Files are selected by extension, ignoring case:

| Extension | Read as |
| --- | --- |
| `.csv` | comma-separated text |
| `.tsv`, `.tab` | tab-separated text |
| `.xlsx`, `.xlsm`, `.xlsb` | Excel workbook |
| `.xls` | Excel 97-2003 workbook |
| `.ods` | OpenDocument spreadsheet |

Other files are ignored, and so are Excel lock files such as `~$Book.xlsx`.
XLSX worksheets are read cell by cell, as a stream.

## Output

The output file name selects the syntax. Add `.gz` to compress it with gzip.

| Output | Meaning |
| --- | --- |
| *(none)* | N-Quads on stdout |
| `*.nq`, `*.nq.gz` | N-Quads, one named graph per source plus one for the run |
| `*.nt`, `*.nt.gz` | N-Triples, the union of all graphs |
| `*.trig`, `*.trig.gz` | TriG |

Any other extension is an error. Output always streams, except with
`--canonical`.

## Canonical output

`--canonical` writes [RDFC-1.0](https://www.w3.org/TR/rdf-canon/) canonical
N-Quads or N-Triples, so that two runs over the same input can be compared
byte for byte. This mode buffers the whole dataset in memory. It does not
support TriG.

```sh
triplify ./dumps --canonical -o raw.nq
```

## Logging and exit status

Logs go to stderr. By default only warnings are shown; `-v` adds progress per
file, `-vv` and `-vvv` add debugging detail, `-q` shows only errors and `-qq`
nothing. The `RUST_LOG` environment variable overrides these flags.

A file that cannot be read is reported as a warning, and the other files are
still triplified. The command then exits with a nonzero status that says how
many sources failed.

## Examples

```sh
# Every table under ./dumps, as compressed N-Quads
triplify ./dumps -o raw.nq.gz

# Two specific files, all graphs merged into N-Triples
triplify orders.csv rates.xlsx -o raw.nt

# The current directory, to stdout
triplify > raw.nq
```
