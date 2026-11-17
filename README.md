# Triplifier

Turn any data dump into raw RDF. No mapping and almost no options.

```sh
triplifier ./dumps -o raw.nq.gz
```

Triplifier walks the given files and directories, honouring `.gitignore`, and
captures every CSV, TSV and spreadsheet table it can read as RDF, following the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl).
Each source gets its own named graph, with a DCAT distribution, an SPDX SHA-256
checksum, its Git origin and a PROV activity for the run.

**Documentation: <https://ekgf.github.io/triplifier/>**

- [Install](https://ekgf.github.io/triplifier/install/)
- [Usage](https://ekgf.github.io/triplifier/usage/)
- [Output model](https://ekgf.github.io/triplifier/output-model/)
- [Why capture first](https://ekgf.github.io/triplifier/why/)
- [Contributing](https://ekgf.github.io/triplifier/contributing/)

The website is built by [GraphArch](https://github.com/EKGF/grapharch) from
`docs/site/` and `docs/grapharch.ttl`.

## Install

```sh
cargo install --locked --git https://github.com/EKGF/triplifier
```

## License

MIT
