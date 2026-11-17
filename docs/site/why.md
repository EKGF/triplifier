---
title: "Why capture first"
description: "Why triplifier captures a data dump as it physically is, and leaves mapping, joining and cleansing to SPARQL, SHACL, OWL and SKOS in the graph."
permalink: /why/
nav_order: 5
grapharch:
  subject: urn:triplifier:website:page:why
---

# Why capture first, map later

Mapping comes after capture, not before it.

## The cost of mapping first

Mapping languages such as [R2RML](https://www.w3.org/TR/r2rml/),
[RML](https://rml.io/specs/rml/), [Ontop](https://ontop-vkg.org/) and
[D2RQ](http://d2rq.org/) ask you to understand the source first. You write SQL
views, joins and functions against a live system, often a production one, and
those queries take its locks and its load. Every source technology needs its
own expertise up front, before a single triple exists.

## Capture the dump instead

Triplifier reverses that. It reads the cheapest and most granular artefact a
system can produce, the data dump (CSV, spreadsheet, database export), offline
and away from production. It turns it into RDF *as it physically is*: sources,
views, columns, rows and values.

## Map in the graph

Everything else happens later, in the graph, with SPARQL, SHACL, OWL and SKOS.
These are the same tools whatever the source, so mapping, joining, cleansing
and versioning become graph work on a captured, immutable snapshot of that
source's truth. Content-addressed identifiers and checksums tie every triple
back to the exact bytes it came from.

## Formulas as rules

Spreadsheet formulas are captured too, including one relative form per
filled-down column. The next step, outside the capture itself, is to compile
these column rules into SHACL:

- `sh:SPARQLRule` (SHACL-AF) to *recompute* a derived column;
- `sh:sparql` constraints to *check* that cached results still agree with
  their inputs;
- cross-sheet references (`'Rates'!R2C3`) become joins between views.

This needs a translator from the Excel function language into SPARQL.
Arithmetic, comparisons, `IF`, `SUM` and `AVERAGE` over ranges and string
functions cover most real workbooks. It is not part of triplifier yet.

## Roadmap

These are plans, not current features:

- PostgreSQL `pg_dump` and `COPY` output, Parquet, Avro and JSON Lines.
- Direct JDBC and ODBC table dumps (full table scans only, no views).
- Compile column formulas to SHACL rules and constraints.
- Upstream the remaining extension terms into the EKGF Raw Data Ontology.
