# Triplifier

Turn any data dump into raw RDF. No mapping and almost no options.

```sh
triplifier ./dumps -o raw.nq.gz
```

The triplifier walks the given files and directories, honouring `.gitignore`,
and captures every table it can read as RDF. The output follows the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl).
Each source gets its own named graph, along with metadata:

- DCAT distribution: media type, byte size, download URL and modification date.
- SPDX SHA-256 checksum.
- Git origin: repository, commit, path, blob, and whether the file differs from `HEAD`.
- A PROV activity for the run.

## Why capture first, map later

Mapping languages such as [R2RML](https://www.w3.org/TR/r2rml/),
[RML](https://rml.io/specs/rml/), [Ontop](https://ontop-vkg.org/) and
[D2RQ](http://d2rq.org/) ask you to understand the source first. You write SQL
views, joins and functions against a live system, often a production one, and
those queries take its locks and its load. Every source technology needs its own
expertise up front, before a single triple exists.

The triplifier reverses that. It reads the cheapest and most granular artefact a
system can produce, the data dump (CSV, spreadsheet, database export), offline
and away from production. It turns it into RDF *as it physically is*:
sources, views, columns, rows and values. Everything else happens later, in the
graph, with SPARQL, SHACL, OWL and SKOS. These are the same tools whatever the
source, so mapping, joining, cleansing and versioning become graph work on a
captured, immutable snapshot of that source's truth.

## What you get

| Source structure | Raw RDF |
| --- | --- |
| the file | `raw:CsvFile` / `raw:XlsxFile` … `raw:File`, `dcat:Distribution`, `prov:Entity` |
| a worksheet / the CSV body | `raw:View` (`raw:viewNumber`, `rdfs:label`) |
| a column | `raw:Column` (`raw:columnNumber`, header text as `rdfs:label`) |
| a data row | `raw:Row` (`raw:viewRowNumber`, 0-based) |
| a non-empty cell | `raw:Value` (`raw:valueOf`, `raw:forColumn`, `raw:literalValue`) |
| an error cell (`#N/A`) | `raw:NullValue` with `raw:sourceText` |

Rules:

- **No guessing.** CSV fields are always `xsd:string`, so `"00123"` stays
  `"00123"`. Spreadsheet cells keep the type the workbook stored them with.
- **Positional identity.** IRIs are derived from the SHA-256 of the content
  plus physical positions, for example `urn:ekgf:raw:source:<sha256>:view:1:row:7:column:3`.
  The same bytes always give the same IRIs, wherever the file lives.
  Identical files found twice are triplified once.
- **Columns are physical.** Header text is only a label, and ragged rows
  just add columns.
- The output conforms to the SHACL shapes in `ekgf-raw.ttl`. Terms not yet
  upstream are in [`docs/ontology/raw-extension.ttl`](docs/ontology/raw-extension.ttl).

## Usage

```text
triplifier [PATHS]... [-o OUTPUT] [--canonical] [--base IRI] [-v]
```

| Output | Meaning |
| --- | --- |
| *(none)* | N-Quads on stdout |
| `*.nq`, `*.nq.gz` | N-Quads, one named graph per source plus one for the run |
| `*.nt`, `*.nt.gz` | N-Triples, the union of all graphs |
| `*.trig`, `*.trig.gz` | TriG |

`--canonical` writes [RDFC-1.0](https://www.w3.org/TR/rdf-canon/) canonical
N-Quads or N-Triples. This mode buffers the dataset in memory; all other modes
stream. `--base` (or `TRIPLIFIER_BASE`) replaces the default `urn:ekgf:raw:`.

Formats read: CSV, TSV, XLSX/XLSM/XLSB, XLS and ODS. XLSX is read cell by
cell (streaming). Files are processed concurrently, one per core, into a single
writer.

## Roadmap

- PostgreSQL `pg_dump` / `COPY` output, Parquet, Avro, JSON Lines.
- Direct JDBC/ODBC table dumps (full table scans only, no views).
- Upstream the extension terms into `ekgf-raw.ttl`.

## License

MIT
