use std::fmt;

use serde::{Deserialize, Serialize};

use crate::asset_events::{AssetEvent, AssetEventList};
use crate::error::Error;
use crate::fifo::{FifoStore, PurchaseInfo};
use crate::fx::FxRates;
use crate::read::KontoauszugData;
use crate::settings::Settings;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Veräußerung {
    datum_zeit: String,
    menge: f64,
    symbol: String,
    währung: String,
    erlös: f64,
    steuern_und_gebühren: f64,
    einstandskosten: f64,
    fx: f64,
}

impl Veräußerung {
    pub fn new(
        datum_zeit: &str,
        menge: f64,
        symbol: &str,
        währung: &str,
        erlös: f64,
        steuern_und_gebühren: f64,
        einstandskosten: f64,
        fx: f64,
    ) -> Self {
        Self {
            datum_zeit: datum_zeit.to_string(),
            menge,
            symbol: symbol.to_string(),
            währung: währung.to_string(),
            erlös,
            steuern_und_gebühren,
            einstandskosten,
            fx: fx,
        }
    }

    fn netto_erlös(&self) -> f64 {
        self.erlös - self.steuern_und_gebühren
    }

    fn eur_gewinn(&self) -> f64 {
        self.fx * self.netto_erlös() - self.einstandskosten
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Veräußerungen {
    veräußerungen: Vec<Veräußerung>,
}

impl fmt::Display for Veräußerungen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sum = 0.0;
        for v in &self.veräußerungen {
            let netto_erlös = v.netto_erlös();
            let eur_gewinn = v.eur_gewinn();
            sum += eur_gewinn;
            writeln!(
                f,
                "Verkauf am {:8} von {:8.2} {:6} zu {:8.2} {} oder {:8.2} EUR mit Einstand {:8.2} EUR und real. GuV {:8.2} EUR",
                v.datum_zeit,
                v.menge,
                v.symbol,
                netto_erlös,
                v.währung,
                v.fx * netto_erlös,
                v.einstandskosten,
                eur_gewinn,
            )?;
        }

        writeln!(f, "Gesamtsumme Kapitalerträge in EUR: {}", sum)?;
        Ok(())
    }
}

impl Veräußerungen {
    fn add(&mut self, v: Veräußerung) {
        self.veräußerungen.push(v);
    }
}

pub fn berechne_veräußerungsgewinne(
    kontoauszug: &KontoauszugData,
    fx_rates: &FxRates,
    fifo: &mut FifoStore,
    settings: &Settings,
) -> Result<(Veräußerungen, Veräußerungen)> {
    let mut aktien_veräußerungen = Veräußerungen::default();
    let mut etf_veräußerungen = Veräußerungen::default();
    // Erstelle Liste mit Käufen/Verkäufen, Transfers und Kapitalmaßnahmen
    let event_list = AssetEventList::von_kontoauszug(kontoauszug)?;
    for (date, events) in event_list.events {
        for event in &events {
            match event {
                AssetEvent::Kauf(t) => {
                    // Käufe in fifo aufnehmen
                    let effektiver_kurs =
                        (t.menge * t.transaktions_kurs + t.prov_gebuehr) / t.menge;
                    fifo.add(&t.symbol, date, PurchaseInfo::new(t.menge, effektiver_kurs))?;
                }
                AssetEvent::Verkauf(t) => {
                    let fx = fx_rates.get_fx_rate(date, &t.waehrung)?;
                    let purchase_cost = fifo.reduce(&t.symbol, date, -t.menge)?;
                    let veräußerung = Veräußerung::new(
                        &t.datum_zeit,
                        -t.menge,
                        &t.symbol,
                        &t.waehrung,
                        t.erloese,
                        t.prov_gebuehr,
                        fx,
                        purchase_cost,
                    );
                    if kontoauszug.is_etf(&t.symbol, None)? {
                        etf_veräußerungen.add(veräußerung);
                    } else {
                        aktien_veräußerungen.add(veräußerung);
                    }
                }
                AssetEvent::Transfer(t) => {
                    if t.richtung != "In" {
                        eprintln!(
                            "Warnung: Transferrichtung '{}' wird aktuell nicht unterstützt, ignoriere Transfer {:?}",
                            t.richtung, t
                        );
                    } else if fifo.contains(&t.symbol) {
                        eprintln!(
                            "Warnung: Transfer von {} in existierende Position wird nicht unterstützt, ignoriere Transfer {:?}",
                            t.symbol, t
                        );
                    } else {
                        if let Some(einstandskosten) = settings.einstandskosten.get(&t.symbol) {
                            fifo.add(
                                &t.symbol,
                                date,
                                PurchaseInfo::new(t.menge, einstandskosten / t.menge),
                            )?;
                        } else {
                            eprintln!(
                                "Warnung: Für Transfer von {} fehlen die Einstandskosten, ignoriere Transfer {:?}",
                                t.symbol, t
                            );
                        }
                    }
                }
                AssetEvent::Kapitalmaßnahme(k) => {
                    fifo.exchange_assets(&k, date)?;
                }
            }
        }
    }

    Ok((aktien_veräußerungen, etf_veräußerungen))
}
