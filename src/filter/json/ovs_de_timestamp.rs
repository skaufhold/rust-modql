use super::ovs_json::OpValueToOpValType;
use crate::filter::{OpValTimestamp, OpValsTimestamp};
use serde::{de::MapAccess, de::Visitor, Deserialize, Deserializer};
use serde_json::Value;
use std::fmt;
use chrono::{DateTime, Utc};

impl<'de> Deserialize<'de> for OpValsTimestamp {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_any(TimestampOpValsVisitor)
	}
}

struct TimestampOpValsVisitor;

impl<'de> Visitor<'de> for TimestampOpValsVisitor {
	type Value = OpValsTimestamp;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		write!(formatter, "TimestampOpValsVisitor visitor not implemented for this type.")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let dt = v.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)?;
		Ok(OpValTimestamp::Eq(dt).into())
	}

	fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let dt = v.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)?;
		Ok(OpValTimestamp::Eq(dt).into())
	}

	fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
	where
		M: MapAccess<'de>,
	{
		let mut opvals: Vec<OpValTimestamp> = Vec::new();

		while let Some(k) = map.next_key::<String>()? {
			let value = map.next_value::<Value>()?;
			let opval = OpValTimestamp::op_value_to_op_val_type(&k, value).map_err(serde::de::Error::custom)?;
			opvals.push(opval)
		}

		Ok(OpValsTimestamp(opvals))
	}
}
