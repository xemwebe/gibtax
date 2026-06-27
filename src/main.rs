use std::env;
use std::error::Error;
mod read;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Kontoauszug.csv".to_string());

    println!("Reading {path} …\n");
    let d = read::parse_kontoauszug(&path)?;

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

    // ── Spot-check: print first transaction ───────────────────────────────────
    if let Some(t) = d.transaktionen.first() {
        println!("First stock transaction:");
        println!(
            "  {:10}  {:6}  {:8}  {}  qty {:.0}  @ {:.4}  proceeds {:.2}",
            t.datum_zeit,
            t.waehrung,
            t.symbol,
            t.vermoegenswert_kategorie,
            t.menge,
            t.transaktions_kurs,
            t.erloese
        );
    }
    if let Some(t) = d.dividenden.first() {
        println!("First dividend:");
        println!(
            "  {}  {:4}  {:.2}  {}",
            t.datum, t.waehrung, t.betrag, t.beschreibung
        );
    }

    Ok(())
}
