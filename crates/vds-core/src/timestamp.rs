//! A UTC timestamp at one-second resolution.
//!
//! Second resolution, not sub-second, and always `Z`, never a numeric offset.
//! Both choices are about reproducibility rather than taste: a proof record is
//! digested, a warrant repeats that digest as evidence, and a timestamp whose
//! textual form varies by platform or locale would move a digest without moving
//! anything a reader would call a fact.

use std::fmt;

use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VdsError};
use crate::schema_util::date_time_string;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// Now, to the second, in UTC.
    pub fn now() -> Self {
        Self(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true))
    }

    pub fn parse(raw: impl Into<String>) -> Result<Self> {
        let raw = raw.into();
        let parsed = DateTime::parse_from_rfc3339(&raw).map_err(|e| {
            VdsError::Identifier(format!("{raw:?} is not an RFC 3339 timestamp: {e}"))
        })?;
        let normalised = parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        if normalised != raw {
            return Err(VdsError::Identifier(format!(
                "{raw:?} is not in the one canonical form VDS writes. Expected {normalised:?}: \
                 UTC, second resolution, trailing Z. A timestamp whose form varies moves a \
                 digest without moving a fact."
            )));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn as_datetime(&self) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&self.0)
            .expect("a Timestamp is parseable by construction")
            .with_timezone(&Utc)
    }

    pub fn plus_seconds(&self, seconds: i64) -> Self {
        Self(
            (self.as_datetime() + chrono::Duration::seconds(seconds))
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    }

    pub fn format(&self, pattern: &str) -> String {
        self.as_datetime().format(pattern).to_string()
    }

    /// Milliseconds from `self` to `other`, floored at zero.
    pub fn millis_until(&self, other: &Timestamp) -> u64 {
        (other.as_datetime() - self.as_datetime())
            .num_milliseconds()
            .max(0) as u64
    }

    /// A fixed timestamp, for tests and for any caller that must not read the
    /// clock. Exposed rather than duplicated in each test module.
    pub fn fixed(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Self {
        Self(
            Utc.with_ymd_and_hms(y, mo, d, h, mi, s)
                .single()
                .expect("a valid fixed instant")
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(raw).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for Timestamp {
    fn schema_name() -> String {
        "Timestamp".to_owned()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        date_time_string("UTC, second resolution, trailing Z. The one canonical form VDS writes.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_second_resolution_utc() {
        let now = Timestamp::now();
        assert!(now.as_str().ends_with('Z'), "{now}");
        assert_eq!(now.as_str().len(), 20, "{now}");
        assert!(
            !now.as_str().contains('.'),
            "no sub-second component: {now}"
        );
    }

    #[test]
    fn parse_refuses_a_non_canonical_form() {
        assert!(Timestamp::parse("2026-07-25T10:00:00Z").is_ok());
        assert!(
            Timestamp::parse("2026-07-25T10:00:00+00:00").is_err(),
            "a numeric zero offset is the same instant in a different form, and two forms \
             of one instant is two digests of one fact"
        );
        assert!(Timestamp::parse("2026-07-25T10:00:00.500Z").is_err());
        assert!(Timestamp::parse("2026-07-25").is_err());
        assert!(Timestamp::parse("not a time").is_err());
    }

    #[test]
    fn arithmetic_stays_canonical() {
        let at = Timestamp::fixed(2026, 7, 25, 23, 59, 59);
        assert_eq!(at.plus_seconds(1).as_str(), "2026-07-26T00:00:00Z");
    }

    #[test]
    fn duration_never_goes_negative() {
        let later = Timestamp::fixed(2026, 7, 25, 10, 0, 1);
        let earlier = Timestamp::fixed(2026, 7, 25, 10, 0, 0);
        assert_eq!(earlier.millis_until(&later), 1000);
        assert_eq!(later.millis_until(&earlier), 0);
    }

    #[test]
    fn round_trips_through_serde() {
        let at = Timestamp::fixed(2026, 7, 25, 10, 0, 0);
        let text = serde_json::to_string(&at).unwrap();
        assert_eq!(text, "\"2026-07-25T10:00:00Z\"");
        assert_eq!(serde_json::from_str::<Timestamp>(&text).unwrap(), at);
    }

    #[test]
    fn deserialisation_refuses_a_non_canonical_form() {
        assert!(serde_json::from_str::<Timestamp>("\"2026-07-25T10:00:00+01:00\"").is_err());
    }
}
