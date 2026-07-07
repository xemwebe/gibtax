//use crate::error::Error;

use std::{
    collections::HashMap,
    fmt::{self, Display},
};

//type Result<T> = std::result::Result<T, Error>;

pub struct Quellensteuer {
    pub beschreibung: String,
    pub datum: String,
    pub währung: String,
    pub betrag: f64,
    pub eur_betrag: f64,
}

#[derive(Default)]
pub struct QuellensteuerPerJurisdiktion {
    // Abegführte Quellenstuer nach Jufisdiktionen
    qsteuer_per_juris: HashMap<String, Vec<Quellensteuer>>,
}

impl QuellensteuerPerJurisdiktion {
    pub fn insert(&mut self, jurisdiktion: String, qtax: Quellensteuer) {
        if let Some(val) = self.qsteuer_per_juris.get_mut(&jurisdiktion) {
            val.push(qtax);
        } else {
            self.qsteuer_per_juris.insert(jurisdiktion, vec![qtax]);
        }
    }
}

impl Display for QuellensteuerPerJurisdiktion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut key_count = 0;
        let mut sum = 0.0;
        if let Some(german_qtax) = self.qsteuer_per_juris.get("DE") {
            key_count += 1;
            writeln!(
                f,
                "Abgeführte deutsche Quellensteuer auf Dividenden (inkl. Solidaritätszuschlag"
            )?;
            for tax in german_qtax {
                writeln!(
                    f,
                    "{:110} {:10} {:3} {:9.2}",
                    tax.beschreibung,
                    tax.datum,
                    tax.währung,
                    (100.0f64 * tax.betrag).round() / 100.0
                )?;
                sum += tax.betrag;
            }
            writeln!(
                f,
                "Gesamtbetrag in EUR: {:9.2}",
                (100.0f64 * sum).round() / 100.
            )?;
        } else {
            writeln!(f, "Es wurden keine deutschen Quellensteuern abgeführt.")?;
        }
        if self.qsteuer_per_juris.len() > key_count {
            writeln!(
                f,
                "\nAbgeführte ausländische Quellensteuer nach Jurisdiction"
            )?;
            for jurisdiction in self.qsteuer_per_juris.keys() {
                if jurisdiction == "DE" {
                    continue;
                }
                writeln!(f, "\nJurisdiction: {jurisdiction}")?;
                let mut waehrung = None;
                let mut eur_sum = 0.0;
                let mut curr_sum = 0.0;
                for tax in &self.qsteuer_per_juris[jurisdiction] {
                    if let Some(waehrung) = waehrung {
                        if waehrung != tax.währung {
                            writeln!(
                                f,
                                "Warnung: Inkonsistente Währung in derseblen Jurisdiction!"
                            )?;
                        }
                    } else {
                        waehrung = Some(tax.währung.as_str());
                    }
                    writeln!(
                        f,
                        "{:110} {:10} {:9.2} {:3} {:9.2} EUR",
                        tax.beschreibung,
                        tax.datum,
                        (100.0f64 * tax.betrag).round() / 100.0,
                        tax.währung,
                        (100.0f64 * tax.eur_betrag).round() / 100.0,
                    )?;
                    curr_sum += tax.betrag;
                    eur_sum += tax.eur_betrag;
                    sum += tax.eur_betrag;
                }
                writeln!(
                    f,
                    "Gesamtbetrag in {}: {:.2} oder {:.2} EUR",
                    waehrung.unwrap_or("unknown"),
                    (100.0f64 * curr_sum).round() / 100.,
                    (100.0f64 * eur_sum).round() / 100.
                )?;
            }
        } else {
            writeln!(
                f,
                "Es wurden keine Quellensteuer für ausländische Jurisdiktionen abgeführt."
            )?;
        }
        if sum != 0.0 {
            writeln!(
                f,
                "Gesamtbetrag über alle Jurisdiktionen (einschl. EUR): {sum:9.2} EUR"
            )?;
        }
        Ok(())
    }
}
