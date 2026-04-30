use crate::filter::OpVal;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct OpValsTimestamp(pub Vec<OpValTimestamp>);

#[derive(Debug, Clone)]
pub enum OpValTimestamp {
	Eq(DateTime<Utc>),
	Not(DateTime<Utc>),

	In(Vec<DateTime<Utc>>),
	NotIn(Vec<DateTime<Utc>>),

	Lt(DateTime<Utc>),
	Lte(DateTime<Utc>),

	Gt(DateTime<Utc>),
	Gte(DateTime<Utc>),

	Null(bool),
}

// region:    --- From ... to OpValTimestamp
impl From<DateTime<Utc>> for OpValTimestamp {
	fn from(val: DateTime<Utc>) -> Self {
		OpValTimestamp::Eq(val)
	}
}

impl From<&DateTime<Utc>> for OpValTimestamp {
	fn from(val: &DateTime<Utc>) -> Self {
		OpValTimestamp::Eq(*val)
	}
}
// endregion: --- From ... to OpValTimestamp

// region:    --- OpValTimestamp to OpVal
impl From<OpValTimestamp> for OpVal {
	fn from(val: OpValTimestamp) -> Self {
		OpVal::Timestamp(val)
	}
}
// endregion: --- OpValTimestamp to OpVal

// region:    --- json
mod json {
	use super::*;
	use crate::filter::json::OpValueToOpValType;
	use crate::{Error, Result};
	use serde_json::Value;

	impl OpValueToOpValType for OpValTimestamp {
		fn op_value_to_op_val_type(op: &str, value: Value) -> Result<Self>
		where
			Self: Sized,
		{
			fn into_datetimes(value: Value) -> Result<Vec<DateTime<Utc>>> {
				let Value::Array(array) = value else {
					return Err(Error::JsonValArrayWrongType { actual_value: value });
				};

				let mut dts = Vec::new();
				for item in array.into_iter() {
					if let Value::String(s) = item {
						let dt = s.parse::<DateTime<Utc>>().map_err(|_| Error::JsonValNotOfType("DateTime<Utc>"))?;
						dts.push(dt);
					} else {
						return Err(Error::JsonValNotOfType("DateTime<Utc>"));
					}
				}
				Ok(dts)
			}

			fn into_datetime(value: Value) -> Result<DateTime<Utc>> {
				let Value::String(s) = value else {
					return Err(Error::JsonValNotOfType("DateTime<Utc>"));
				};
				s.parse::<DateTime<Utc>>().map_err(|_| Error::JsonValNotOfType("DateTime<Utc>"))
			}

			let ov = match (op, value) {
				("$eq", v) => OpValTimestamp::Eq(into_datetime(v)?),
				("$not", v) => OpValTimestamp::Not(into_datetime(v)?),

				("$in", value) => OpValTimestamp::In(into_datetimes(value)?),
				("$notIn", value) => OpValTimestamp::NotIn(into_datetimes(value)?),

				("$lt", v) => OpValTimestamp::Lt(into_datetime(v)?),
				("$lte", v) => OpValTimestamp::Lte(into_datetime(v)?),
				("$gt", v) => OpValTimestamp::Gt(into_datetime(v)?),
				("$gte", v) => OpValTimestamp::Gte(into_datetime(v)?),

				("$null", Value::Bool(v)) => OpValTimestamp::Null(v),

				(_, v) => {
					return Err(Error::JsonOpValNotSupported {
						operator: op.to_string(),
						value: v,
					});
				}
			};
			Ok(ov)
		}
	}
}
// endregion: --- json

// region:    --- with-sea-query
#[cfg(feature = "with-sea-query")]
mod with_sea_query {
	use super::*;
	use crate::filter::{FilterNodeOptions, SeaResult, sea_is_col_value_null};
	use crate::{into_node_column_expr, into_node_value_expr};
	use sea_query::{BinOper, ColumnRef, Condition, ExprTrait, SimpleExpr};

	impl OpValTimestamp {
		pub fn into_sea_cond_expr(self, col: &ColumnRef, node_options: &FilterNodeOptions) -> SeaResult<Condition> {
			let binary_fn = |op: BinOper, dt: DateTime<Utc>| -> SeaResult<Condition> {
				let vxpr = into_node_value_expr(dt, node_options);
				let column = into_node_column_expr(col.clone(), node_options);
				Ok(SimpleExpr::binary(column, op, vxpr).into())
			};

			let binaries_fn = |op: BinOper, dts: Vec<DateTime<Utc>>| -> SeaResult<Condition> {
				let vxpr_list: Vec<SimpleExpr> = dts
					.into_iter()
					.map(|v| into_node_value_expr(v, node_options))
					.collect();
				let vxpr = SimpleExpr::Tuple(vxpr_list);
				let column = into_node_column_expr(col.clone(), node_options);
				Ok(SimpleExpr::binary(column, op, vxpr).into())
			};

			let cond = match self {
				OpValTimestamp::Eq(dt) => binary_fn(BinOper::Equal, dt)?,
				OpValTimestamp::Not(dt) => binary_fn(BinOper::NotEqual, dt)?,
				OpValTimestamp::In(dts) => binaries_fn(BinOper::In, dts)?,
				OpValTimestamp::NotIn(dts) => binaries_fn(BinOper::NotIn, dts)?,
				OpValTimestamp::Lt(dt) => binary_fn(BinOper::SmallerThan, dt)?,
				OpValTimestamp::Lte(dt) => binary_fn(BinOper::SmallerThanOrEqual, dt)?,
				OpValTimestamp::Gt(dt) => binary_fn(BinOper::GreaterThan, dt)?,
				OpValTimestamp::Gte(dt) => binary_fn(BinOper::GreaterThanOrEqual, dt)?,
				OpValTimestamp::Null(null) => sea_is_col_value_null(col.clone(), null),
			};

			Ok(cond)
		}
	}
}
// endregion: --- with-sea-query
