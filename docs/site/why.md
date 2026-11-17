---
title: "Why capture first"
description: "Why triplify captures a data dump as it physically is, and leaves mapping, joining and cleansing to SPARQL, SHACL, OWL and SKOS in the graph."
permalink: /why/
nav_order: 5
grapharch:
  subject: urn:triplify:website:page:why
---

# Why capture first, map later

Triplify first, think later. Mapping comes after capture, not before it.

## The cost of mapping first

Mapping languages such as [R2RML](https://www.w3.org/TR/r2rml/),
[RML](https://rml.io/specs/rml/), [Ontop](https://ontop-vkg.org/) and
[D2RQ](http://d2rq.org/) ask you to understand the source first. You write SQL
views, joins and functions against a live system, often a production one, and
those queries take its locks and its load. Every source technology needs its
own expertise up front, before a single triple exists.

## Capture the dump instead

Triplify reverses that. It reads the cheapest and most granular artefact a
system can produce, the data dump (CSV, spreadsheet, database export), offline
and away from production. It turns it into RDF *as it physically is*: sources,
views, columns, rows and values.

It does this deterministically. The same bytes always give the same triples
with the same identifiers. It is always the same program, with no mapping
files, connection strings or per-source configuration, so installing and
running it needs no knowledge from an operator or DevOps engineer. All
optimisation effort goes into one job: getting the data out of its source
quickly and completely.

## Mapping is iterative

A real mapping is tedious work. The meaning of the data has to be reverse
engineered and then mapped properly onto the target ontologies. That takes
discussions with subject-matter experts, DBAs, end users, data architects,
ontologists and taxonomists, and a lot of trial and error. Getting it right can
easily take more than a hundred runs.

None of those runs should touch a production system, or any live system. Each
run against a live source adds load and annoyance, and the waste piles up with
every iteration. With triplify, the source is read once. Every later iteration
runs against the captured raw RDF.

## Map in the graph

Everything else happens later, in the graph, with SPARQL, SHACL, OWL and SKOS.
These are the same tools whatever the source, so mapping, joining, cleansing
and versioning become graph work on a captured, immutable snapshot of that
source's truth. Content-addressed identifiers and checksums tie every triple
back to the exact bytes it came from.

Starting from raw RDF is far more efficient:

- **Batch and parallel.** Mapping runs are ordinary graph jobs. You can run
  them in batch, in parallel and as often as needed, disconnected from any
  source system.
- **Correlate early.** Raw RDF can be joined directly with other datasets,
  including other raw captures, reference data and existing knowledge graphs,
  before any mapping is decided.
- **Map into a separate graph.** Mapped data goes straight into the target
  ontologies, in a named graph of its own. The raw RDF stays untouched in its
  staging store, so every mapped triple can be traced back to it and remapped
  when understanding improves.

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
functions cover most real workbooks. It is not part of triplify yet.

## Roadmap

These are plans, not current features:

- PostgreSQL `pg_dump` and `COPY` output, Parquet, Avro and JSON Lines.
- Direct JDBC and ODBC table dumps (full table scans only, no views).
- Compile column formulas to SHACL rules and constraints.
