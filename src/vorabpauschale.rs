use crate::{
    error::{Error, Result},
    read::KontoauszugData,
};

use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum FondsTyp {
    Aktien,
    Mixed,
}

#[derive(Debug)]
pub struct EtfPosition {
    menge: f64,
    eur_betrag: f64,
    fonds_typ: FondsTyp,
    dividenden: f64,
    monate: u8,
}

impl EtfPosition {
    pub fn new(menge: f64, eur_betrag: f64, fonds_typ: FondsTyp, monate: u8) -> Self {
        Self {
            menge,
            eur_betrag,
            fonds_typ,
            dividenden: 0.0,
            monate,
        }
    }
}

#[derive(Debug, Default)]
pub struct EtfPositionen {
    positionen: HashMap<String, Vec<EtfPosition>>,
}

impl EtfPositionen {
    pub fn add_position(&mut self, symbol: &str, position: EtfPosition) {
        if self.positionen.contains_key(symbol) {
            if let Some(pos) = self.positionen.get_mut(symbol) {
                pos.push(position);
            }
        } else {
            self.positionen.insert(symbol.to_owned(), vec![position]);
        }
    }

    pub fn get_positionen(&self, symbol: &str) -> Option<&Vec<EtfPosition>> {
        self.positionen.get(symbol)
    }

    pub fn get_alle_positionen(&self) -> &HashMap<String, Vec<EtfPosition>> {
        &self.positionen
    }

    pub fn update_unterjährig(&mut self, kontoauszug: &KontoauszugData) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct VorabpauschaleInfo {
    /// Symbol
    symbol: String,
    /// Fondswert zu Beginn der Periode (Jahresanfang oder Kaufdatum falls jünger)
    start_wert: f64,
    /// Fondswert zum Edne der Periode
    end_wert: f64,
    /// Basiszinssatz in %
    base_rate: f64,
    /// Typ des Fonds
    fonds_typ: FondsTyp,
    /// ausgesschüttete Dividenden zwischen Start und Ende der Periode
    dividenden: f64,
    /// Angrebochene Anzahl der Monate, die der Fonds gehalten wurden (12 für ein ganzes Jahr)
    monate: u8,
}

impl VorabpauschaleInfo {
    pub fn sammle_vorabpauschalen_infos(
        last_kontoauszug: &KontoauszugData,
        kontoauszug: &KontoauszugData,
        base_rate: f64,
    ) -> Result<Vec<VorabpauschaleInfo>> {
        let mut pauschalen_infos = Vec::new();
        let mut offene_positionen_start = last_kontoauszug.get_open_etf_positions()?;
        offene_positionen_start.update_unterjährig(kontoauszug)?;
        let offene_positionen_ende = kontoauszug.get_open_etf_positions()?;
        for (etf, position) in offene_positionen_ende.get_alle_positionen() {
            if position.is_empty() {
                continue;
            }
            if position.len() != 1 {
                return Err(Error::EtfEndPositionUngleichEins(etf.to_owned()));
            }
            let position = &position[0];
            if let Some(start_positionen) = offene_positionen_start.get_positionen(&etf) {
                for start_position in start_positionen {
                    pauschalen_infos.push(VorabpauschaleInfo {
                        symbol: etf.to_string(),
                        start_wert: start_position.eur_betrag,
                        end_wert: position.eur_betrag,
                        base_rate,
                        fonds_typ: position.fonds_typ,
                        dividenden: start_position.dividenden,
                        monate: start_position.monate,
                    });
                }
            } else {
                return Err(Error::PassendeEtfStartPositionFehlt(etf.to_string()));
            }
        }
        Ok(pauschalen_infos)
    }

    pub fn calc(&self) -> Result<f64> {
        let basisertrag =
            (self.start_wert * self.base_rate / 100.0 * 0.7) * (self.monate as f64) / 12.0;
        let performance = self.end_wert / self.start_wert - 1.0;
        let basisertrag = basisertrag.min(performance);
        let effektiver_basisertrag = (basisertrag - self.dividenden).max(0.0);
        let teilfreistellung = match self.fonds_typ {
            FondsTyp::Aktien => 0.3,
            FondsTyp::Mixed => 0.15,
        };
        let tax = effektiver_basisertrag * (1.0 - teilfreistellung) * 0.25 * 1.055;
        Ok(tax)
    }
}
