use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unvollständige oder fehlerhafte Kauftransaktion")]
    InvalidBuyTransaction,
    #[error("Unvollständige oder fehlerhafte Verkaufstransaktion")]
    InvalidSellTransaction,
    #[error("Wechselkurs für {0} nicht gefunden")]
    CurrencyNotFoundError(String),
    #[error("Fehler beim Lesen der CSV-Datei")]
    CsvError(#[from] csv::Error),
    #[error("Record nicht gefunden in CSV-Datei")]
    RecordNotFound,
    #[error("Umwandeln von String in Zahl fehlgeschlagen")]
    ParsingNumberFailed,
    #[error("Umwandeln von String in Datum fehlgeschlagen")]
    FailedToParseDate,
    #[error("Umwandeln von Timestamp in Datum fehlgeschlagen")]
    FailedToConvertDate,
}
