use crate::date::convert_date;
use crate::error::Error;
use crate::fx::FxRates;
use crate::read::KontoauszugData;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Dividende {
    pub beschreibung: String,
    pub date: String,
    pub betrag: f64,
    pub währung: String,
    pub eur_betrag: f64,
    pub is_etf: bool,
}

pub fn berechne_dividenden(
    kontoauszug: &KontoauszugData,
    fx_rates: &FxRates,
) -> Result<Vec<Dividende>> {
    let mut dividenden = Vec::new();
    for div in &kontoauszug.dividenden {
        let date = convert_date(&div.datum)?;
        let fx = fx_rates.get_fx_rate(date, &div.waehrung)?;
        let eur_betrag = fx * div.betrag;
        let isin = parse_isin(&div.beschreibung)?;
        dividenden.push(Dividende {
            beschreibung: div.beschreibung.clone(),
            date: div.datum.clone(),
            betrag: (100.0 * div.betrag).round() / 100.0,
            währung: div.waehrung.clone(),
            eur_betrag: (100.0 * eur_betrag).round() / 100.0,
            is_etf: kontoauszug.is_etf(&isin)?,
        });
    }
    Ok(dividenden)
}

fn parse_isin(beschreibung: &str) -> Result<String> {
    let re = regex::Regex::new(r"^[A-Za-z0-9]*\(([A-Z0-9]*)\)").unwrap();
    if let Some(caps) = re.captures(beschreibung) {
        Ok(caps[1].to_string())
    } else {
        Err(Error::FailedToParseSymboleFromDescription)
    }
}
