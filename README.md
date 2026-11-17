# Triplify

Turn any data dump into raw RDF. No mapping and almost no options.

```sh
triplify ./dumps -o raw.nq.gz
```

Triplify walks the given files and directories, honouring `.gitignore`, and
captures every CSV, TSV and spreadsheet table it can read as RDF, following the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl).
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
