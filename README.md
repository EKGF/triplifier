# Triplify

Turn any data dump into raw RDF. No mapping and almost no options.

```sh
triplify ./dumps -o raw.nq.gz
```

Triplify walks the given files and directories, honouring `.gitignore`,
and captures every table it can read as RDF. The output follows the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl).
Each source gets its own named graph, along with metadata:

- DCAT distribution: media type, byte size, download URL and modification date.
- SPDX SHA-256 checksum.
- Git origin: repository, commit, path, blob, and whether the file differs from `HEAD`.
- A PROV activity for the run.

## Install

```sh
uv tool install triplify     # or: pipx install triplify / pip install triplify
cargo install triplify       # from source
```

The PyPI package is a [maturin](https://www.maturin.rs/) `bin` wheel that
contains the native executable, the same as GraphArch's. Build one locally with
`uvx maturin build --release --locked --out dist`.

## Why capture first, map later

Mapping languages such as [R2RML](https://www.w3.org/TR/r2rml/),
[RML](https://rml.io/specs/rml/), [Ontop](https://ontop-vkg.org/) and
[D2RQ](http://d2rq.org/) ask you to understand the source first. You write SQL
views, joins and functions against a live system, often a production one, and
those queries take its locks and its load. Every source technology needs its own
expertise up front, before a single triple exists.

Triplify reverses that. It reads the cheapest and most granular artefact a
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
| a formula cell | the cached result as above, plus `raw:formula` (A1) and `raw:formulaR1C1` |
| a column filled with one formula | `raw:formulaR1C1` on the `raw:Column` |

Rules:

- **No guessing.** CSV fields are always `xsd:string`, so `"00123"` stays
  `"00123"`. Spreadsheet cells keep the type the workbook stored them with.
- **Positional, content-addressed identity.** Every IRI is a `urn:uuid:` that
  anyone can recompute from the source bytes and a position. See
  [Identifiers](#identifiers). Identical files found twice are triplified once.
- **Columns are physical.** Header text is only a label, and ragged rows
  just add columns.
- Every term comes from the
  [EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl),
  and the output conforms to its SHACL `ValidationShapeSet`.

## Usage

```text
triplify [PATHS]... [-o OUTPUT] [--canonical] [-v]
```

| Output | Meaning |
| --- | --- |
| *(none)* | N-Quads on stdout |
| `*.nq`, `*.nq.gz` | N-Quads, one named graph per source plus one for the run |
| `*.nt`, `*.nt.gz` | N-Triples, the union of all graphs |
| `*.trig`, `*.trig.gz` | TriG |

`--canonical` writes [RDFC-1.0](https://www.w3.org/TR/rdf-canon/) canonical
N-Quads or N-Triples. This mode buffers the dataset in memory; all other modes
stream.

Formats read: CSV, TSV, XLSX/XLSM/XLSB, XLS and ODS. XLSX is read cell by
cell (streaming). Files are processed concurrently, one per core, into a single
writer.

## Formulas

Formulas are data too. Each formula cell keeps its cached result as
`raw:literalValue`, or becomes a `raw:NullValue` when the workbook stored no
result. It also keeps the formula verbatim (`raw:formula "A2*B2"`) and the same
formula rewritten relative to its own cell (`raw:formulaR1C1 "RC[-2]*RC[-1]"`).
When every formula in a column has the same relative form, which is the usual
case for a filled-down formula, the column itself gets that `raw:formulaR1C1`.
At that point the column is a derivation rule
(`total = qty × price`) rather than a list of numbers.

The next step, outside the capture step itself, is to compile these column
rules into SHACL:

- `sh:SPARQLRule` (SHACL-AF) to *recompute* a derived column;
- `sh:sparql` constraints to *check* that cached results still agree with
  their inputs;
- cross-sheet references (`'Rates'!R2C3`) become joins between views.

This needs a translator for the Excel function language into SPARQL.
Arithmetic, comparisons, `IF`, `SUM`/`AVERAGE` over ranges and string
functions cover most real workbooks.

## Identifiers

Every resource has a canonical *name*. Its IRI is the name-based
**UUIDv8 with SHA-256** of [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562)
(§6.5, Appendix B.2):

```text
digest = SHA-256( NAMESPACE.bytes ‖ UTF-8(name) )
uuid   = digest[0..16] with version := 8, variant := 0b10
iri    = urn:uuid:<lowercase hyphenated uuid>

NAMESPACE = uuidv8_sha256(NameSpace_URL, "https://ekgf.org/ontology/raw#")
```

Names (`<sha256>` is the lowercase hex SHA-256 of the source bytes; integers are
decimal with no leading zeros):

| Resource | Name |
| --- | --- |
| source | `source/<sha256>` |
| its named graph | `source/<sha256>/graph` |
| its checksum | `source/<sha256>/checksum` |
| view (1-based) | `source/<sha256>/view/<v>` |
| column (1-based) | `source/<sha256>/view/<v>/column/<c>` |
| row (0-based) | `source/<sha256>/view/<v>/row/<r>` |
| value | `source/<sha256>/view/<v>/row/<r>/column/<c>` |
| software agent | `agent/triplify/<version>` |

A run is an event, not content. It gets a time-ordered UUIDv7, and its graph is
`uuidv8_sha256(<run uuid>, "graph")`.

## Roadmap

- PostgreSQL `pg_dump` / `COPY` output, Parquet, Avro, JSON Lines.
- Direct JDBC/ODBC table dumps (full table scans only, no views).
- Compile column formulas to SHACL rules and constraints (see [Formulas](#formulas)).
- Upstream the extension terms into `ekgf-raw.ttl`.

## License

MIT
