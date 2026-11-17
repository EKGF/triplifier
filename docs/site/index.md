---
title: "Triplifier"
description: "Turn any data dump into raw RDF. Triplifier captures CSV, TSV and spreadsheet files as the EKGF Raw Data Ontology, with DCAT, SPDX, Git and PROV provenance, and no mapping."
permalink: /
nav_order: 1
grapharch:
  subject: urn:triplifier:website:page:home
  hero: landing
  panel_sections: [what-it-does]
  feature_sections: [principles]
  card_icons:
    capture-never-map: {name: layers, tone: primary}
    content-addressed-identity: {name: file-box, tone: accent}
    provenance-included: {name: git-branch, tone: primary}
    built-for-big-dumps: {name: database, tone: accent}
---

EKGF tool

# Triplifier

Turn any data dump into raw RDF. No mapping and almost no options.

[Install](install.md) [Usage](usage.md) [Source on GitHub](https://github.com/EKGF/triplifier)

## What it does

```sh
triplifier ./dumps -o raw.nq.gz
```

Triplifier walks the files and directories you give it, honouring
`.gitignore`, and captures every table it can read as RDF. It reads CSV, TSV,
XLSX, XLSM, XLSB, XLS and ODS files. The output follows the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl):
sources, views, columns, rows and values, exactly as they physically are.

Each source gets its own named graph, along with its metadata: a DCAT
distribution, an SPDX SHA-256 checksum, its Git origin, and a PROV activity for
the run. See [Output model](output-model.md) for the details.

## Principles

### Capture, never map

Readers report physical structure and the source's own value types. Triplifier
never infers datatypes from text, never picks key columns and never renames
anything. Mapping happens later, in the graph, with SPARQL and SHACL.

### Content-addressed identity

Every IRI is a `urn:uuid:` that anyone can recompute from the source bytes and
a position. Identical files found twice are triplified once.

### Provenance included

Every source records its media type, byte size, location, checksum and Git
blob. Inside a Git work tree it also records the repository, commit and path,
and whether the file differs from `HEAD`.

### Built for big dumps

Files are processed concurrently, one per core, into a single writer. Output
streams, optionally gzip-compressed, unless you ask for canonical RDF.

## Learn more

- [Why capture first](why.md) explains the approach and how it differs from
  mapping languages.
- [Usage](usage.md) is the command-line reference.
- [Contributing](contributing.md) covers the architecture, conventions and
  releases.
