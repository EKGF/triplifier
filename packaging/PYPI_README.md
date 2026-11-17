# Triplify

Turn every CSV, TSV, XLSX, XLS and ODS file under a directory into raw RDF,
using the EKGF Raw Data Ontology. Each source gets its own named graph, with
DCAT, PROV, checksum and git provenance. There is no mapping step. The package
installs the `triplify` command.

```sh
uv tool install triplify
triplify ./dumps -o raw.nq.gz
```

Or run it once in an isolated environment:

```sh
uvx triplify ./dumps -o raw.nt.gz
```

The wheel contains the native executable, so it needs no Python dependencies
and downloads nothing at runtime. Wheels are built for Linux (glibc 2.28+),
macOS 11+ and Windows, on x86-64 and ARM64.

See the [repository](https://github.com/EKGF/triplify) for the output model
and the IRI scheme.
