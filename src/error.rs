use std::num::ParseIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unvollständige oder fehlerhafte Kauftransaktion")]
    InvalidBuyTransaction,
    #[error("Unvollständige oder fehlerhafte Verkaufstransaktion")]
    InvalidSellTransaction,
    #[error("Wechselkurs für {0} nicht gefunden")]
    CurrencyNotFound(String),
    #[error("Fehler beim Lesen der CSV-Datei")]
    Csv(#[from] csv::Error),
    #[error("Record nicht gefunden in CSV-Datei")]
    RecordNotFound,
    #[error("Umwandeln von String in Zahl fehlgeschlagen")]
    ParsingNumberFailed,
    #[error("Umwandeln von String in Datum fehlgeschlagen")]
    FailedToParseDate,
    #[error("Umwandeln von Timestamp in Datum fehlgeschlagen")]
    FailedToConvertDate,
    #[error("Symbol {0} nicht gefunden in Finanzinstrumenten")]
    SymbolNotFound(String),
    #[error("Ungültiger Montsname {0}")]
    InvalidMonthName(String),
    #[error("Invalid integer")]
    ParseIntError(#[from] ParseIntError),
    #[error("Datum des Kontoauszugs konnte nicht gefunden werden")]
    DateNotFound,
    #[error("Parsen eines Symbols von einer Beschreibung ist fehlgeschlagen")]
    FailedToParseSymboleFromDescription,
    #[error("Parsen der Jurisdiktion aus Bechreibung '{0}' fehlgeschlagen")]
    FailedToParseJurisdiction(String),
}
