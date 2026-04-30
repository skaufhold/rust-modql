use crate::filter::OpVal;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OpValsUuid(pub Vec<OpValUuid>);

#[derive(Debug, Clone)]
pub enum OpValUuid {
	Eq(Uuid),
	Not(Uuid),

	In(Vec<Uuid>),
	NotIn(Vec<Uuid>),

	Lt(Uuid),
	Lte(Uuid),

	Gt(Uuid),
	Gte(Uuid),

	Null(bool),
}

// region:    --- From ... to OpValUuid
impl From<Uuid> for OpValUuid {
	fn from(val: Uuid) -> Self {
		OpValUuid::Eq(val)
	}
}

impl From<&Uuid> for OpValUuid {
	fn from(val: &Uuid) -> Self {
		OpValUuid::Eq(*val)
	}
}
// endregion: --- From ... to OpValUuid

// region:    --- OpValUuid to OpVal
impl From<OpValUuid> for OpVal {
	fn from(val: OpValUuid) -> Self {
		OpVal::Uuid(val)
	}
}
// endregion: --- OpValUuid to OpVal

// region:    --- json
mod json {
	use super::*;
	use crate::filter::json::OpValueToOpValType;
	use crate::{Error, Result};
	use serde_json::Value;

	impl OpValueToOpValType for OpValUuid {
		fn op_value_to_op_val_type(op: &str, value: Value) -> Result<Self>
		where
			Self: Sized,
		{
			fn into_uuids(value: Value) -> Result<Vec<Uuid>> {
				let Value::Array(array) = value else {
					return Err(Error::JsonValArrayWrongType { actual_value: value });
				};

				let mut uuids = Vec::new();
				for item in array.into_iter() {
					if let Value::String(s) = item {
						let uuid = Uuid::parse_str(&s).map_err(|_| Error::JsonValNotOfType("Uuid"))?;
						uuids.push(uuid);
					} else {
						return Err(Error::JsonValNotOfType("Uuid"));
					}
				}
				Ok(uuids)
			}

			fn into_uuid(value: Value) -> Result<Uuid> {
				let Value::String(s) = value else {
					return Err(Error::JsonValNotOfType("Uuid"));
				};
				Uuid::parse_str(&s).map_err(|_| Error::JsonValNotOfType("Uuid"))
			}

			let ov = match (op, value) {
				("$eq", v) => OpValUuid::Eq(into_uuid(v)?),
				("$not", v) => OpValUuid::Not(into_uuid(v)?),

				("$in", value) => OpValUuid::In(into_uuids(value)?),
				("$notIn", value) => OpValUuid::NotIn(into_uuids(value)?),

				("$lt", v) => OpValUuid::Lt(into_uuid(v)?),
				("$lte", v) => OpValUuid::Lte(into_uuid(v)?),
				("$gt", v) => OpValUuid::Gt(into_uuid(v)?),
				("$gte", v) => OpValUuid::Gte(into_uuid(v)?),

				("$null", Value::Bool(v)) => OpValUuid::Null(v),

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
	use sea_query::{BinOper, ColumnRef, Condition, ExprTrait, SimpleExpr, Value};

	impl OpValUuid {
		pub fn into_sea_cond_expr(self, col: &ColumnRef, node_options: &FilterNodeOptions) -> SeaResult<Condition> {
			let binary_fn = |op: BinOper, uuid: Uuid| -> SeaResult<Condition> {
				let vxpr = into_node_value_expr(Value::Uuid(Some(uuid)), node_options);
				let column = into_node_column_expr(col.clone(), node_options);
				Ok(SimpleExpr::binary(column, op, vxpr).into())
			};

			let binaries_fn = |op: BinOper, uuids: Vec<Uuid>| -> SeaResult<Condition> {
				let vxpr_list: Vec<SimpleExpr> = uuids
					.into_iter()
					.map(|v| into_node_value_expr(Value::Uuid(Some(v)), node_options))
					.collect();
				let vxpr = SimpleExpr::Tuple(vxpr_list);
				let column = into_node_column_expr(col.clone(), node_options);
				Ok(SimpleExpr::binary(column, op, vxpr).into())
			};

			let cond = match self {
				OpValUuid::Eq(uuid) => binary_fn(BinOper::Equal, uuid)?,
				OpValUuid::Not(uuid) => binary_fn(BinOper::NotEqual, uuid)?,
				OpValUuid::In(uuids) => binaries_fn(BinOper::In, uuids)?,
				OpValUuid::NotIn(uuids) => binaries_fn(BinOper::NotIn, uuids)?,
				OpValUuid::Lt(uuid) => binary_fn(BinOper::SmallerThan, uuid)?,
				OpValUuid::Lte(uuid) => binary_fn(BinOper::SmallerThanOrEqual, uuid)?,
				OpValUuid::Gt(uuid) => binary_fn(BinOper::GreaterThan, uuid)?,
				OpValUuid::Gte(uuid) => binary_fn(BinOper::GreaterThanOrEqual, uuid)?,
				OpValUuid::Null(null) => sea_is_col_value_null(col.clone(), null),
			};

			Ok(cond)
		}
	}
}
// endregion: --- with-sea-query
