# Triplify

Turn CSV files and spreadsheets into RDF, preserving their original structure and values.

Capture your source data without defining a mapping first. Use SPARQL and SHACL
afterward to transform, connect and validate it.

```sh
triplify ./dumps -o raw.nq.gz
```

Triplify walks the given files and directories, honouring `.gitignore`, and
captures every CSV, TSV and spreadsheet table it can read as RDF, following the
[EKGF Raw Data Ontology](https://ekgf.github.io/ekg-ontologies/spec/main/ekg-raw/index.html).
Each source gets its own named graph, with a DCAT distribution, an SPDX SHA-256
checksum, its Git origin and a PROV activity for the run.

**Documentation: <https://ekgf.github.io/triplify/>**

- [Install](https://ekgf.github.io/triplify/install/)
- [Usage](https://ekgf.github.io/triplify/usage/)
- [Output model](https://ekgf.github.io/triplify/output-model/)
- [Why capture first](https://ekgf.github.io/triplify/why/)
- [Contributing](https://ekgf.github.io/triplify/contributing/)

The website is built by [GraphArch](https://github.com/EKGF/grapharch) from
`docs/site/` and `docs/grapharch.ttl`.

## Install

No release is published yet, so install from source:

```sh
cargo install --locked --git https://github.com/EKGF/triplify
```

Once the first release is published, `uv tool install triplify`,
`pipx install triplify`, `pip install triplify` and `cargo install triplify`
install the `triplify` command.

## License

MIT
