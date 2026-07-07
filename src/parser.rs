use crate::error::Error;

type Result<T> = std::result::Result<T, Error>;

pub fn parse_asset_ids(beschreibung: &str) -> Result<(String, String)> {
    let re = regex::Regex::new(r"^([A-Za-z0-9]*)\(([A-Z0-9]*)\)").unwrap();
    if let Some(caps) = re.captures(beschreibung) {
        Ok((caps[1].to_string(), caps[2].to_string()))
    } else {
        Err(Error::FailedToParseSymboleFromDescription)
    }
}
