use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    str::FromStr,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FxError {
    #[error("Wechselkurs für {0} nicht gefunden")]
    CurrencyNotFoundError(String),
}

#[derive(Debug, Default)]
pub struct FxRates {
    tables: HashMap<String, FxTable>,
}

#[derive(Debug, Default)]
pub struct FxTable {
    table: BTreeMap<i64, f64>,
}

impl FxRates {
    pub fn get_fx_rate(&self, date: i64, währung: &str) -> Result<f64> {
        if währung == "EUR" {
            Ok(1.0)
        } else {
            if let Some(fx) = self
                .tables
                .get(währung)
                .ok_or(FxError::CurrencyNotFoundError(währung.to_string()))?
                .get_fx_rate(date)
            {
                Ok(1.0 / fx)
            } else {
                Err(FxError::CurrencyNotFoundError(währung.to_string()).into())
            }
        }
    }
}

impl FxTable {
    pub fn get_fx_rate(&self, date: i64) -> Option<f64> {
        if let Some(fx) = self.table.get(&date) {
            return Some(*fx);
        }
        self.table.range(..date).next_back().map(|(_, f)| *f)
    }
}

pub fn read_fx_rates(fx_path: &Path) -> Result<FxRates> {
    println!("Reading fx rates from '{}'", fx_path.display());
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(fx_path)
        .context(format!("Failed to parse fx rates"))?;
    let mut fx_rates = FxRates::default();
    let mut header_ref = HashMap::<String, usize>::new();
    let headers = rdr.headers().context("Header not found")?;
    for (idx, header) in headers.iter().enumerate() {
        let column_name = header.to_owned();
        if column_name.is_empty() || column_name == "Date" {
            continue;
        }
        header_ref.insert(column_name.clone(), idx);
        fx_rates.tables.insert(column_name, FxTable::default());
    }
    for record in rdr.records() {
        let record = record?;
        let date = record.get(0).to_owned().context("Row without date found")?;
        let date = convert_date(date)?;
        for header in header_ref.keys() {
            if let Some(fx_rate) = record.get(header_ref[header])
                && fx_rate != "N/A"
            {
                if let Some(fx_table) = fx_rates.tables.get_mut(header) {
                    fx_table.table.insert(date.to_owned(), fx_rate.parse()?);
                }
            }
        }
    }
    Ok(fx_rates)
}

/// convert naive date string into days since epoch
pub fn convert_date(date: &str) -> Result<i64> {
    let date = NaiveDate::from_str(&date[0..10]).context("failed to parse date")?;
    let timestamp = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    Ok(timestamp / 86400)
}
