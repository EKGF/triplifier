---
title: "Output model"
description: "What triplify emits: one named graph per source with the EKGF Raw Data Ontology, DCAT distribution metadata, SPDX SHA-256 checksums, Git origin, a PROV run graph, and recomputable urn:uuid identifiers."
permalink: /output-model/
nav_order: 4
grapharch:
  subject: urn:triplify:website:page:output-model
---

# Output model

The output follows the
[EKGF Raw Data Ontology](https://github.com/EKGF/ekg-ontologies/blob/main/ekgf-raw.ttl)
and conforms to its SHACL shapes.

## Named graphs

Every source file gets its own named graph, with its structure and its
metadata. One more graph describes the run. N-Quads and TriG keep these
graphs; N-Triples writes their union.

## Source structure

| Source structure | Raw RDF |
| --- | --- |
| the file | `raw:CsvFile`, `raw:TsvFile` or `raw:XlsxFile`, and always `raw:File`, `raw:Source`, `dcat:Distribution`, `prov:Entity` |
| a worksheet or the CSV body | `raw:View` (`raw:viewNumber`, sheet name as `rdfs:label`) |
| a column | `raw:Column` (`raw:columnNumber`, header text as `rdfs:label`) |
| a data row | `raw:Row` (`raw:viewRowNumber`, 0-based) |
| a non-empty cell | `raw:Value` (`raw:valueOf`, `raw:forColumn`, `raw:literalValue`) |
| an error cell (`#N/A`) | `raw:NullValue` with `raw:sourceText` |
| a formula cell | the cached result as above, plus `raw:formula` (A1) and `raw:formulaR1C1` |
| a column filled with one formula | `raw:formulaR1C1` on the `raw:Column` |

XLS and ODS files are typed `raw:File`. The first physical row of each view is
its header, and the rows after it are data.

## Capture rules

- **No guessing.** CSV fields are always `xsd:string`, so `"00123"` stays
  `"00123"`. Empty fields produce no value. Spreadsheet cells keep the type
  the workbook stored them with: string, integer, double, boolean, date-time or
  duration.
- **Columns are physical.** Header text is only a label, and ragged rows just
  add columns.
- **Duplicates once.** Identical files found twice are triplified once.

A two-row CSV file, `id,qty` and `00123,4`, produces these values (TriG, IRIs
shortened):

```turtle
<urn:uuid:a4bc…> a raw:Row ;
    raw:isRowInView <urn:uuid:d0af…> ;
    raw:viewRowNumber 0 .
<urn:uuid:1ae9…> a raw:Value ;
    raw:valueOf <urn:uuid:a4bc…> ;
    raw:forColumn <urn:uuid:906c…> ;
    raw:literalValue "00123" .
<urn:uuid:86ef…> a raw:Value ;
    raw:valueOf <urn:uuid:a4bc…> ;
    raw:forColumn <urn:uuid:da82…> ;
    raw:literalValue "4" .
```

## Distribution metadata

Each source is a `dcat:Distribution` with:

- `raw:fileName` and `dcterms:title`: the file name;
- `dcat:downloadURL` and `prov:atLocation`: its `file:` URL;
- `dcat:mediaType`: its IANA media type;
- `dcat:byteSize` and `dcterms:modified`.

## Checksums

The source bytes are read once, in a single streaming pass, for both the
SHA-256 digest and the Git blob id. The digest is an `spdx:Checksum` with
`spdx:algorithm spdx:checksumAlgorithm_sha256` and an `xsd:hexBinary`
`spdx:checksumValue`. The Git blob id is `raw:gitBlob`, also for files outside
a Git repository.

## Git origin

For a file inside a Git work tree, the source also records:

- `raw:gitRepository`: the fetch URL of the default remote, and
  `prov:wasDerivedFrom` that URL when it is a valid IRI;
- `raw:gitCommit`: the `HEAD` commit;
- `raw:gitPath`: the path relative to the work tree;
- `raw:gitModified`: whether the bytes read differ from that path in `HEAD`.

## Run provenance

The run graph holds a `prov:Activity` with its start and end times, associated
with a `prov:SoftwareAgent` for this triplify version. The activity
`prov:used` every source. Each source graph is a `dcat:Dataset` that
`prov:wasDerivedFrom` its source and `prov:wasGeneratedBy` the run.

## Formulas

Formulas are data too. Each formula cell keeps its cached result as
`raw:literalValue`, or becomes a `raw:NullValue` when the workbook stored no
result. It also keeps the formula verbatim (`raw:formula "A2*B2"`) and the same
formula rewritten relative to its own cell (`raw:formulaR1C1 "RC[-2]*RC[-1]"`).
When every formula in a column has the same relative form, which is the usual
case for a filled-down formula, the column itself gets that `raw:formulaR1C1`.
The column is then a derivation rule (`total = qty × price`) rather than a list
of numbers.

## Identifiers

Every resource has a canonical *name*. Its IRI is the name-based UUIDv8 with
SHA-256 of [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562) (section 6.5,
appendix B.2):

```text
digest = SHA-256( NAMESPACE.bytes ‖ UTF-8(name) )
uuid   = digest[0..16] with version := 8, variant := 0b10
iri    = urn:uuid:<lowercase hyphenated uuid>

NAMESPACE = uuidv8_sha256(NameSpace_URL, "https://ekgf.org/ontology/raw#")
```

In the names, `<sha256>` is the lowercase hex SHA-256 of the source bytes.
Integers are decimal with no leading zeros.

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
