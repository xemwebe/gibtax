use crate::cash::CashFlow;
use crate::date::convert_timestamp_to_date_string;
use crate::error::Result;
use crate::fifo::FifoStore;
use crate::fifo::PurchaseInfo;
use crate::fx::FxRates;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Währungsverkauf {
    datum: String,
    währung: String,
    erlös: f64,
    fx: f64,
    einstandskosten: f64,
}

impl Währungsverkauf {
    fn eur_erlös(&self) -> f64 {
        self.fx * self.erlös
    }

    fn eur_gewinn(&self) -> f64 {
        self.eur_erlös() - self.einstandskosten
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WährungsVerkäufe {
    verkäufe: Vec<Währungsverkauf>,
}

impl WährungsVerkäufe {
    pub fn parse(
        cash_flows: &[CashFlow],
        fx_rates: &FxRates,
        fifo: &mut FifoStore,
    ) -> Result<Self> {
        let mut verkäufe = WährungsVerkäufe::default();
        for c in cash_flows {
            if c.curr == "EUR" {
                // Keine Währungsgewinne aus EUR-Positionen
                continue;
            }
            let fx = fx_rates.get_fx_rate(c.date, &c.curr)?;
            // Nur Verkäufe sind relevant
            if c.amount >= 0.0 {
                // Käufe in fifo aufnehmen
                fifo.add(&c.curr, c.date, PurchaseInfo::new(c.amount, fx))?;
                continue;
            }
            let einstandskosten = fx * fifo.reduce(&c.curr, c.date, -c.amount)?;
            let verkauf = Währungsverkauf {
                datum: convert_timestamp_to_date_string(c.date)?,
                währung: c.curr.clone(),
                erlös: -fx * c.amount,
                fx,
                einstandskosten,
            };
            verkäufe.add(verkauf);
        }
        Ok(verkäufe)
    }

    fn add(&mut self, verkauf: Währungsverkauf) {
        self.verkäufe.push(verkauf)
    }
}

impl fmt::Display for WährungsVerkäufe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n== Gewinne und Verluste aus Währungsverkäufen")?;
        let mut sum = 0.0;
        for c in &self.verkäufe {
            let eur_gewinn = c.eur_gewinn();
            sum += eur_gewinn;
            writeln!(
                f,
                "Verkauf am {} von {:8.2} {} ({:8.2} EUR) mit Einstand {:8.2} EUR und real. GuV {:8.2} EUR",
                c.datum,
                c.erlös,
                c.währung,
                c.eur_erlös(),
                c.einstandskosten,
                eur_gewinn,
            )?;
        }

        writeln!(f, "Gesamtsumme Kapitalerträge in EUR: {}", sum)?;
        Ok(())
    }
}
