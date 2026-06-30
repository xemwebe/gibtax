use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::{collections::HashMap, error::Error, path::PathBuf};

mod error;
mod fifo;
mod fx;
mod read;
mod read_transactions;

use crate::read_transactions::BuySell;

#[derive(Parser, Debug)]
#[command(name = "gibtax")]
struct Cli {
    /// Zeige ein paar statische Daten zu den eingelesenen Kontoauszügen an
    #[arg(short, long)]
    statistic: bool,

    /// Calculate FIFO information from transaction history
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Berechne FIFO Liste der Trades Calculate FIFO stack based on trade history
    Fifo(FifoArgs),
    /// Erstelle Report als Grundlage für deutsche Steuererklärung
    Report(ReportArgs),
}

#[derive(Debug, Args)]
struct FifoArgs {
    /// Pfad zu einer oder mehreren Transaction-History-Dateien (AllTransactions*.csv)
    #[arg(short, long)]
    transactions: Vec<PathBuf>,
    /// Pfad zu einem Kontoauszug, dessen offene Position zur Initialisizerung der FiFo-Historie dienen
    #[arg(short, long)]
    initial_positions: Option<PathBuf>,
    /// Nur Transactions vor diesem Datum werden berücksichtigt
    #[arg(short, long)]
    max_time: String,
    /// Pfad in den der FIFO status geschrieben werden soll
    #[arg(short, long)]
    out_file: PathBuf,
    /// Pfad zu Datei mit den EZB-Referenzwechselkurshistorie
    #[arg(short, long)]
    fx_rates: PathBuf,
}

#[derive(Debug, Args)]
struct ReportArgs {
    /// Pfad zur Kontoauszugsdatei
    #[arg(short, long)]
    konto_auszug: PathBuf,
    /// Pfad zu Datei mit den EZB-Referenzwechselkurshistorie
    #[arg(short, long)]
    fx_rates: PathBuf,
    /// FIFO-Stand als Basis zur Berechnung der Veräußerungsgewinne
    #[arg(short = 'F', long)]
    fifo_state: Option<PathBuf>,
    /// FIFO-Stand als Basis zur Berechnung der Veräußerungsgewinne
    #[arg(short, long)]
    output_fifo_state: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Report(args) => {
            println!("Reading {} …\n", args.konto_auszug.display());
            let d = read::parse_kontoauszug(&args.konto_auszug)?;
            let fx_rates = fx::read_fx_rates(&args.fx_rates)?;

            if cli.statistic {
                print_statistic(&d);
                return Ok(());
            }
            println!("\nSteueraufstellung");
            println!("");
            print_total_interest(&d.zinsen);
            println!("");
            print_dividenden(&d.dividenden, &fx_rates)?;
            println!("");
            print_quellensteuer(&d.quellensteuer, &fx_rates)?;
            println!("");
            let mut fifo = if let Some(fifo_state) = args.fifo_state {
                let fifo_file = std::fs::File::open(&fifo_state)?;
                let fifo: fifo::FifoStore = serde_json::from_reader(&fifo_file)?;
                fifo
            } else {
                fifo::FifoStore::new(0)
            };
            let mut transactions = d.transaktionen;
            transactions.sort_by(|x, y| {
                if x.datum_zeit == y.datum_zeit {
                    if x.menge > 0.0 {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                } else {
                    x.datum_zeit.cmp(&y.datum_zeit)
                }
            });
            print_aktien_verkäufe(&transactions, &fx_rates, &mut fifo)?;
            if let Some(fifo_output) = args.output_fifo_state {
                let out_file = std::fs::File::create(&fifo_output)?;
                serde_json::to_writer_pretty(&out_file, &fifo)?;
            }
        }
        Commands::Fifo(args) => {
            if cli.statistic {
                for path in args.transactions {
                    println!("Reading transaction history {} …", path.display());
                    let th = read_transactions::parse_transaction_history(&path)?;
                    print_transaction_statistic(&th);
                }
                return Ok(());
            }
            let fx_rates = fx::read_fx_rates(&args.fx_rates)?;
            let mut fifo = if let Some(initial_postions) = args.initial_positions {
                println!("Reading {} …\n", initial_postions.display());
                let d = read::parse_kontoauszug(&initial_postions)?;
                let timestamp = d.get_timestamp()?;
                fifo::FifoStore::from_open_positions(&d.offene_positionen, timestamp, &fx_rates)?
            } else {
                fifo::FifoStore::new(0)
            };
            let mut fifo_transactions = Vec::new();
            for path in args.transactions {
                println!("Reading transaction history {} …", path.display());
                let th = read_transactions::parse_transaction_history(&path)?;
                let mut transactions = th.extract_purchase_infos(&fx_rates)?;
                fifo_transactions.append(&mut transactions);
            }
            fifo_transactions.sort();
            println!("fifo_transactions: {fifo_transactions:?}");
            let max_time_stamp = fx::convert_date(&args.max_time)?;
            for transaction in fifo_transactions {
                println!("process transaction: {transaction:?}");
                if transaction.get_timestamp() >= fifo.get_timestamp()
                    && transaction.get_timestamp() < max_time_stamp
                {
                    match transaction.buy_sell {
                        BuySell::Buy => {
                            fifo.add(
                                &transaction.symbol,
                                transaction.timestamp,
                                fifo::PurchaseInfo::new(transaction.quantity, transaction.price),
                            )?;
                        }
                        BuySell::Sell => {
                            let _ = fifo.reduce(
                                &transaction.symbol,
                                transaction.timestamp,
                                transaction.quantity,
                            )?;
                        }
                    }
                }
            }
            let out_file = std::fs::File::create(&args.out_file)?;
            serde_json::to_writer_pretty(out_file, &fifo)?;
        }
    }

    Ok(())
}

fn print_aktien_verkäufe(
    transactions: &[read::TransaktionRow],
    fx_rates: &fx::FxRates,
    fifo: &mut fifo::FifoStore,
) -> Result<()> {
    println!("Gewinne und Verluste aus Aktienverkäufen");
    let mut sum = 0.0;
    for t in transactions {
        // Nur Verkäufe sind relevant
        let date = fx::convert_date(&t.datum_zeit)?;
        if t.menge >= 0.0 {
            // Käufe in fifo aufnehmen
            let effektiver_kurs = (t.menge * t.transaktions_kurs + t.prov_gebuehr) / t.menge;
            fifo.add(
                &t.symbol,
                date,
                fifo::PurchaseInfo::new(t.menge, effektiver_kurs),
            )?;
            continue;
        }
        let fx = fx_rates.get_fx_rate(date, &t.waehrung)?;
        let purchase_cost = fifo.reduce(&t.symbol, date, -t.menge)?;
        let eur_betrag = fx * (t.erloese + t.prov_gebuehr) - purchase_cost;
        sum += eur_betrag;
        println!(
            "Verkauf am {:8} von {:8.2} {:6} zu {:8.2} {} oder {:8.2} EUR mit Einstand {:8.2} EUR und real. GuV {:8.2} EUR",
            t.datum_zeit,
            -t.menge,
            t.symbol,
            t.erloese + t.prov_gebuehr,
            t.waehrung,
            fx * (t.erloese + t.prov_gebuehr),
            purchase_cost,
            eur_betrag,
        )
    }

    println!("Gesamtsumme Kapitalerträge in EUR: {}", sum);
    Ok(())
}

fn print_total_interest(zinsen: &[read::ZinsRow]) {
    for z in zinsen {
        if z.beschreibung == "Gesamt Zinsen in EUR" {
            println!("Realisierte Zinsen: {}", z.betrag);
            return;
        }
    }
    println!("#Fehler#: Gesamt Zinsen in EUR nicht gefunden");
}

fn print_dividenden(dividends: &[read::DividendeRow], fx_rates: &fx::FxRates) -> Result<()> {
    println!("Erhaltene Dividenden");
    let mut last_curr = "".to_string();
    let mut curr_sum = 0.0;
    let mut eur_curr_sum = 0.0;
    let mut eur_sum = 0.0;
    for div in dividends {
        if last_curr != div.waehrung {
            if last_curr != "" {
                println!(
                    "Summe Dividenden in {last_curr}: {} {last_curr} oder {} EUR\n",
                    (100.0f64 * curr_sum).round() / 100.0,
                    (100.0f64 * eur_curr_sum).round() / 100.0,
                );
            }
            last_curr = div.waehrung.clone();
            curr_sum = 0.0;
            eur_curr_sum = 0.0;
        }
        let date = fx::convert_date(&div.datum)?;
        let fx = fx_rates.get_fx_rate(date, &div.waehrung)?;
        let eur_betrag = fx * div.betrag;
        curr_sum += div.betrag;
        eur_sum += eur_betrag;
        eur_curr_sum += eur_betrag;
        println!(
            "{:110} {:10} {:9.2} {:3} {:9.2} EUR",
            div.beschreibung,
            div.datum,
            (100.0 * div.betrag).round() / 100.0,
            div.waehrung,
            (100.0f64 * eur_betrag).round() / 100.0,
        );
    }
    println!(
        "Summe aller Dividenden in EUR: {}",
        (100.0 * eur_sum).round() / 100.0
    );
    Ok(())
}

fn print_quellensteuer(qsteuern: &[read::QuellensteuerRow], fx_rates: &fx::FxRates) -> Result<()> {
    let mut qtax_by_jurisdiction: HashMap<String, Vec<read::QuellensteuerRow>> = HashMap::new();
    let re = regex::Regex::new(r"- (.{2}) Steuer$").unwrap();
    for tax in qsteuern {
        if let Some(caps) = re.captures(&tax.beschreibung) {
            let jurisdiction = caps[1].to_string();
            if let Some(val) = qtax_by_jurisdiction.get_mut(&jurisdiction) {
                val.push(tax.clone());
            } else {
                qtax_by_jurisdiction.insert(jurisdiction, vec![tax.clone()]);
            }
        }
    }

    println!("Abgeführte deutsche Quellensteuer auf Dividenden (inkl. Solidaritätszuschlag");
    let mut sum = 0.0;
    for tax in &qtax_by_jurisdiction["DE"] {
        println!(
            "{:110} {:10} {:3} {:9.2}",
            tax.beschreibung,
            tax.datum,
            tax.waehrung,
            (100.0f64 * tax.betrag).round() / 100.0
        );
        sum += tax.betrag;
    }
    println!(
        "Gesamtbetrag in EUR: {:9.2}",
        (100.0f64 * sum).round() / 100.
    );

    println!("\nAbgeführte ausländische Quellensteuer nach Jurisdiction");
    for jurisdiction in qtax_by_jurisdiction.keys() {
        if jurisdiction == "DE" {
            continue;
        }
        println!("\nJurisdiction: {jurisdiction}");
        let mut waehrung = None;
        let mut eur_sum = 0.0;
        let mut curr_sum = 0.0;
        for tax in &qtax_by_jurisdiction[jurisdiction] {
            if let Some(waehrung) = waehrung {
                if waehrung != &tax.waehrung {
                    println!("Warnung: Inkonsistente Währung in derseblen Jurisdiction!");
                }
            } else {
                waehrung = Some(tax.waehrung.as_str());
            }
            let date = fx::convert_date(&tax.datum)?;
            let fx = fx_rates.get_fx_rate(date, &tax.waehrung)?;
            let eur_betrag = fx * tax.betrag;
            println!(
                "{:110} {:10} {:9.2} {:3} {:9.2} EUR",
                tax.beschreibung,
                tax.datum,
                (100.0f64 * tax.betrag).round() / 100.0,
                tax.waehrung,
                (100.0f64 * eur_betrag).round() / 100.0,
            );
            curr_sum += tax.betrag;
            eur_sum += eur_betrag;
            sum += eur_betrag;
        }
        println!(
            "Gesamtbetrag in {}: {:.2} oder {:.2} EUR",
            waehrung.unwrap_or("unknown"),
            (100.0f64 * curr_sum).round() / 100.,
            (100.0f64 * eur_sum).round() / 100.
        );
    }
    println!("Gesamtbetrag über alle Jurisdiktionen (einschl. EUR): {sum:9.2} EUR");
    Ok(())
}

fn print_transaction_statistic(th: &read_transactions::TransactionHistoryData) {
    println!("\n  Transaction History");
    println!("  {}", "─".repeat(66));
    println!("  {:<58} {:>6}", "Statement", th.statement.len());
    println!("  {:<58} {:>6}", "Summary", th.summary.len());
    println!(
        "  {:<58} {:>6}",
        "Transaction History",
        th.transactions.len()
    );
    println!("  {}", "─".repeat(66));

    // Count by transaction type
    let mut by_type: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in &th.transactions {
        *by_type.entry(t.transaction_type.as_str()).or_default() += 1;
    }
    let mut types: Vec<(&str, usize)> = by_type.into_iter().collect();
    types.sort_by_key(|(name, _)| *name);
    println!("  Transaction types:");
    for (name, count) in types {
        println!("    {:<54} {:>6}", name, count);
    }
    println!();
}

fn print_statistic(d: &read::KontoauszugData) {
    // ── Summary table ─────────────────────────────────────────────────────────
    macro_rules! row {
        ($label:expr, $vec:expr) => {
            println!("  {:<58} {:>6}", $label, $vec.len());
        };
    }

    println!("  {:<58} {:>6}", "Table", "Rows");
    println!("  {}", "─".repeat(66));
    row!("Statement", d.statement);
    row!("Kontoinformation", d.kontoinformation);
    row!("Nettovermögenswert", d.nettovermoegenswert);
    row!("  └─ Zeitgewichtete Rendite", d.zeitgewichtete_rendite);
    row!("Veränderung des NAV", d.veraenderung_nav);
    row!("Mark-to-Market-Performance-Überblick", d.mtm_performance);
    row!(
        "Übersicht realisierte/unrealisierte Perf.",
        d.performance_uebersicht
    );
    row!("Cash-Bericht", d.cash_bericht);
    row!("Offene Positionen", d.offene_positionen);
    row!("Devisenpositionen", d.devisenpositionen);
    row!("Netto-Aktienpositionsübersicht", d.netto_aktien);
    row!("Transaktionen (Wertpapiere)", d.transaktionen);
    row!("Transaktionen (Devisen)", d.devisen_transaktionen);
    row!("Transaktionsgebühren", d.transaktionsgebuehren);
    row!("Kapitalmaßnahmen", d.kapitalmassnnahmen);
    row!("Einzahlungen & Auszahlungen", d.ein_auszahlungen);
    row!("Dividenden", d.dividenden);
    row!("Quellensteuer", d.quellensteuer);
    row!("Zinsen", d.zinsen);
    row!("Aufgelaufene Zinsen", d.aufgelaufene_zinsen);
    row!("Veränderung im Dividendenanfall", d.dividendenanfall);
    row!("SYEP Securities Lent", d.syep_lent);
    row!("SYEP Securities Lent Activity", d.syep_activity);
    row!("SYEP Securities Lent Interest Details", d.syep_interest);
    row!("Informationen zum Finanzinstrument", d.finanzinstrumente);
    row!("Codes", d.codes);
    row!("Hinweise/Rechtshinweise", d.hinweise);
    println!("  {}", "─".repeat(66));
    println!("  Gesamt-G&V: {:.2} EUR\n", d.gesamt_guv);
}
