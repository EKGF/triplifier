//! IRIs of every vocabulary term triplify emits.
//!
//! All `raw` terms are defined, with SHACL shapes, in
//! <https://ekgf.github.io/ekg-ontologies/spec/main/ekg-raw/index.html>.

macro_rules! terms {
    ($ns:literal; $($name:ident = $local:literal),* $(,)?) => {
        $(pub const $name: ::oxrdf::NamedNodeRef<'static> =
            ::oxrdf::NamedNodeRef::new_unchecked(concat!($ns, $local));)*
    };
}

pub mod raw {
    pub const NS: &str = "https://ekgf.org/ontology/raw#";
    terms! { "https://ekgf.org/ontology/raw#";
        SOURCE = "Source", FILE = "File", XLSX_FILE = "XlsxFile",
        VIEW = "View", COLUMN = "Column", ROW = "Row", VALUE = "Value",
        NULL_VALUE = "NullValue",
        HAS_VIEW = "hasView", IS_VIEW_IN = "isViewIn",
        HAS_COLUMN = "hasColumn", IS_COLUMN_IN_VIEW = "isColumnInView",
        IS_ROW_IN_VIEW = "isRowInView", VALUE_OF = "valueOf",
        FOR_COLUMN = "forColumn", LITERAL_VALUE = "literalValue",
        SOURCE_TEXT = "sourceText", COLUMN_NUMBER = "columnNumber",
        VIEW_ROW_NUMBER = "viewRowNumber", FILE_NAME = "fileName",
        CSV_FILE = "CsvFile", TSV_FILE = "TsvFile",
        VIEW_NUMBER = "viewNumber",
        GIT_REPOSITORY = "gitRepository", GIT_COMMIT = "gitCommit",
        GIT_PATH = "gitPath", GIT_BLOB = "gitBlob",
        GIT_MODIFIED = "gitModified",
        FORMULA = "formula", FORMULA_R1C1 = "formulaR1C1",
        COUNT = "count", NULL_COUNT = "nullCount", UNIQUE = "unique",
        TOP = "top", FREQ = "freq", INFERRED_DATATYPE = "inferredDatatype",
        CONFORMING_COUNT = "conformingCount", MIN = "min", MAX = "max",
        MEAN = "mean", STD = "std", P25 = "25pct", P50 = "50pct",
        P75 = "75pct",
    }
}

pub mod rdf {
    terms! { "http://www.w3.org/1999/02/22-rdf-syntax-ns#"; TYPE = "type" }
}

pub mod rdfs {
    terms! { "http://www.w3.org/2000/01/rdf-schema#"; LABEL = "label" }
}

pub mod prov {
    terms! { "http://www.w3.org/ns/prov#";
        ENTITY = "Entity", ACTIVITY = "Activity",
        SOFTWARE_AGENT = "SoftwareAgent",
        USED = "used", WAS_GENERATED_BY = "wasGeneratedBy",
        WAS_DERIVED_FROM = "wasDerivedFrom",
        WAS_ASSOCIATED_WITH = "wasAssociatedWith",
        STARTED_AT_TIME = "startedAtTime", ENDED_AT_TIME = "endedAtTime",
        AT_LOCATION = "atLocation",
    }
}

pub mod dcat {
    terms! { "http://www.w3.org/ns/dcat#";
        DATASET = "Dataset", DISTRIBUTION = "Distribution",
        DISTRIBUTION_P = "distribution", MEDIA_TYPE = "mediaType",
        BYTE_SIZE = "byteSize", DOWNLOAD_URL = "downloadURL",
    }
}

pub mod dcterms {
    terms! { "http://purl.org/dc/terms/";
        MODIFIED = "modified", TITLE = "title", IDENTIFIER = "identifier",
    }
}

pub mod spdx {
    terms! { "http://spdx.org/rdf/terms#";
        CHECKSUM_CLASS = "Checksum", CHECKSUM = "checksum",
        ALGORITHM = "algorithm", CHECKSUM_VALUE = "checksumValue",
        SHA256 = "checksumAlgorithm_sha256",
    }
}

pub mod xsd {
    terms! { "http://www.w3.org/2001/XMLSchema#";
        STRING = "string", INTEGER = "integer", DOUBLE = "double",
        BOOLEAN = "boolean", DECIMAL = "decimal", DATE = "date", DATE_TIME = "dateTime", DURATION = "duration",
        NON_NEGATIVE_INTEGER = "nonNegativeInteger",
        HEX_BINARY = "hexBinary",
    }
}
