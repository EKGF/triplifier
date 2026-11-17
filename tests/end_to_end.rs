//! Triplify a small directory and check the raw shape of the output.

use {
    oxrdf::{Dataset, NamedNodeRef, QuadRef},
    oxrdfio::{RdfFormat, RdfParser},
    std::{io::Read, path::Path},
    triplifier::{
        Triplifier,
        vocab::{dcat, raw, rdf, xsd},
        writer::Output,
    },
};

fn fixtures(dir: &Path) {
    std::fs::write(
        dir.join("people.csv"),
        "\u{feff}id,name,code\n1,Alice,00123\n2,,7,extra\n",
    )
    .unwrap();
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet().set_name("Orders").unwrap();
    ws.write_string(0, 0, "order").unwrap();
    ws.write_string(0, 1, "amount").unwrap();
    ws.write_string(0, 2, "paid").unwrap();
    ws.write_number(1, 0, 42).unwrap();
    ws.write_number(1, 1, 9.5).unwrap();
    ws.write_boolean(1, 2, true).unwrap();
    wb.add_worksheet().set_name("Empty").unwrap();
    wb.save(dir.join("orders.xlsx")).unwrap();
    // Excel lock file and an unsupported file are ignored.
    std::fs::write(dir.join("~$orders.xlsx"), b"junk").unwrap();
    std::fs::write(dir.join("notes.txt"), b"hello").unwrap();
}

async fn run(dir: &Path, out: &Path, canonical: bool) -> triplifier::Summary {
    Triplifier::new(
        vec![dir.to_owned()],
        Output::file(out.to_owned(), canonical).unwrap(),
    )
    .run()
    .await
    .unwrap()
}

fn load(bytes: &[u8], format: RdfFormat) -> Dataset {
    RdfParser::from_format(format)
        .for_slice(bytes)
        .collect::<Result<Dataset, _>>()
        .unwrap()
}

fn count(ds: &Dataset, p: NamedNodeRef<'_>, o: NamedNodeRef<'_>) -> usize {
    ds.iter()
        .filter(|q: &QuadRef<'_>| q.predicate == p && q.object == o.into())
        .count()
}

#[tokio::test]
async fn csv_and_xlsx_to_gzipped_nquads() {
    let input = tempfile::tempdir().unwrap();
    fixtures(input.path());
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("raw.nq.gz");
    let summary = run(input.path(), &out, false).await;
    assert_eq!((summary.sources, summary.failed), (2, 0));

    let mut nq = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(&out).unwrap())
        .read_to_end(&mut nq)
        .unwrap();
    let ds = load(&nq, RdfFormat::NQuads);
    assert_eq!(ds.len() as u64, summary.quads);

    assert_eq!(count(&ds, rdf::TYPE, raw::CSV_FILE), 1);
    assert_eq!(count(&ds, rdf::TYPE, raw::XLSX_FILE), 1);
    assert_eq!(count(&ds, rdf::TYPE, dcat::DATASET), 2);
    assert_eq!(count(&ds, rdf::TYPE, raw::VIEW), 3);
    // CSV: 3 header columns + 1 ragged; XLSX: 3.
    assert_eq!(count(&ds, rdf::TYPE, raw::COLUMN), 7);
    assert_eq!(count(&ds, rdf::TYPE, raw::ROW), 3);
    // CSV: 1,Alice,00123,2,7,extra ; XLSX: 42,9.5,true
    assert_eq!(count(&ds, rdf::TYPE, raw::VALUE), 9);

    let literals: Vec<_> = ds
        .iter()
        .filter(|q| q.predicate == raw::LITERAL_VALUE)
        .filter_map(|q| {
            match q.object {
                oxrdf::TermRef::Literal(l) => {
                    Some((l.value().to_owned(), l.datatype().into_owned()))
                },
                _ => None,
            }
        })
        .collect();
    assert!(
        literals.contains(&("00123".into(), xsd::STRING.into_owned())),
        "no guessing"
    );
    assert!(
        literals.contains(&("1".into(), xsd::STRING.into_owned())),
        "BOM stripped from header only"
    );
    assert!(
        literals.contains(&("42".into(), xsd::DOUBLE.into_owned())),
        "{literals:?}"
    );
    assert!(literals.contains(&("true".into(), xsd::BOOLEAN.into_owned())));
}

#[tokio::test]
async fn canonical_ntriples_is_sorted_and_deterministic() {
    let input = tempfile::tempdir().unwrap();
    fixtures(input.path());
    let out_dir = tempfile::tempdir().unwrap();
    let a = out_dir.path().join("a.nt");
    let b = out_dir.path().join("b.nt");
    run(input.path(), &a, true).await;
    run(input.path(), &b, true).await;
    // Everything except the run (a UUIDv7 event) must be identical.
    let strip_run = |p: &Path| -> Vec<String> {
        let text = std::fs::read_to_string(p).unwrap();
        let run = text
            .lines()
            .find(|l| l.ends_with("<http://www.w3.org/ns/prov#Activity> ."))
            .and_then(|l| l.split_whitespace().next())
            .unwrap()
            .to_owned();
        text.lines()
            .filter(|l| !l.contains(&run))
            .map(str::to_owned)
            .collect()
    };
    let (la, lb) = (strip_run(&a), strip_run(&b));
    assert!(!la.is_empty());
    assert!(
        la.windows(2).all(|w| w[0] < w[1]),
        "sorted and unique"
    );
    assert_eq!(la, lb, "same input, same canonical source graphs");
}
