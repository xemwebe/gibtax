use crate::error::Error;
use chrono::{DateTime, TimeZone, Utc};

type Result<T> = std::result::Result<T, Error>;

pub fn convert_timestamp_to_date_string(timestamp: i64) -> Result<String> {
    let datetime: DateTime<Utc> = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .ok_or(Error::FailedToConvertDate)?;
    Ok(datetime.format("%Y-%m-%d").to_string())
}
