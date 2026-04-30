use super::ovs_json::OpValueToOpValType;
use crate::filter::{OpValUuid, OpValsUuid};
use serde::{de::MapAccess, de::Visitor, Deserialize, Deserializer};
use serde_json::Value;
use std::fmt;

impl<'de> Deserialize<'de> for OpValsUuid {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_any(UuidOpValsVisitor)
	}
}

struct UuidOpValsVisitor;

impl<'de> Visitor<'de> for UuidOpValsVisitor {
	type Value = OpValsUuid;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		write!(formatter, "UuidOpValsVisitor visitor not implemented for this type.")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let uuid = uuid::Uuid::parse_str(v).map_err(serde::de::Error::custom)?;
		Ok(OpValUuid::Eq(uuid).into())
	}

	fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let uuid = uuid::Uuid::parse_str(&v).map_err(serde::de::Error::custom)?;
		Ok(OpValUuid::Eq(uuid).into())
	}

	fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
	where
		M: MapAccess<'de>,
	{
		let mut opvals: Vec<OpValUuid> = Vec::new();

		while let Some(k) = map.next_key::<String>()? {
			let value = map.next_value::<Value>()?;
			let opval = OpValUuid::op_value_to_op_val_type(&k, value).map_err(serde::de::Error::custom)?;
			opvals.push(opval)
		}

		Ok(OpValsUuid(opvals))
	}
}
