//! Protobuf (proto3) message types and conversions for modql filter and list-options types.
//!
//! Add this crate alongside `modql`:
//!
//! ```toml
//! [dependencies]
//! modql       = { version = "..." }
//! modql-proto = { version = "..." }
//! ```
//!
//! The canonical `.proto` schema lives at `proto/modql.proto` in this crate.
//! Feed it to `protoc` (or any gRPC toolchain such as `tonic-build`) to generate
//! client stubs in any supported language that can talk to a Rust gRPC service
//! using these types.
//!
//! # Type mapping
//!
//! | modql type | proto message |
//! |---|---|
//! | `modql::filter::FilterGroups` | [`ProtoFilterGroups`] |
//! | `modql::filter::FilterGroup` | [`ProtoFilterGroup`] (via `Vec<Vec<FilterNode>>`) |
//! | `modql::filter::FilterNode` | [`ProtoFilterNode`] |
//! | `modql::filter::FilterNodeOptions` | [`ProtoFilterNodeOptions`] |
//! | `modql::filter::OpVal` | [`ProtoOpVal`] (oneof) |
//! | `modql::filter::OpValString` | [`ProtoOpValString`] (30-variant oneof) |
//! | `modql::filter::OpValInt64` | [`ProtoOpValInt64`] |
//! | `modql::filter::OpValInt32` | [`ProtoOpValInt32`] |
//! | `modql::filter::OpValFloat64` | [`ProtoOpValFloat64`] |
//! | `modql::filter::OpValBool` | [`ProtoOpValBool`] |
//! | `modql::filter::OpValValue` | [`ProtoOpValJsonValue`] (values JSON-encoded as strings) |
//! | `modql::filter::ListOptions` | [`ProtoListOptions`] |
//! | `modql::filter::OrderBys` | [`ProtoOrderBys`] |
//! | `modql::filter::OrderBy` | [`ProtoOrderBy`] (oneof `asc`/`desc`) |
//!
//! # Conversion direction
//!
//! This crate implements **inbound deserialization** (proto → modql) only.
//! Use [`TryFrom`] / [`TryInto`] for types that can fail (missing `oneof` fields)
//! and plain [`From`] for infallible conversions:
//!
//! ```ignore
//! use prost::Message as _;
//! use modql_proto::{ProtoFilterGroups, ProtoListOptions};
//! use modql::filter::{FilterGroups, ListOptions};
//!
//! // Receive raw bytes from a tonic gRPC handler, then:
//! let proto_fg = ProtoFilterGroups::decode(request_bytes)?;
//! let filter: FilterGroups = proto_fg.try_into()?;
//!
//! let proto_lo = ProtoListOptions::decode(opts_bytes)?;
//! let list_opts: ListOptions = proto_lo.into();
//! ```
//!
//! # Errors
//!
//! [`ProtoConversionError`] is returned when a required `oneof` field is absent in
//! the received message, or when a JSON-encoded value inside [`ProtoOpValJsonValue`]
//! cannot be parsed by `serde_json`.
//!
//! # JSON-encoded values (`OpValValue`)
//!
//! `modql::filter::OpValValue` wraps arbitrary `serde_json::Value`s, which have no
//! direct proto3 equivalent. They are transmitted as JSON strings inside
//! [`ProtoOpValJsonValue`]: the sender serialises the value with `serde_json::to_string`
//! and the receiver parses it back.
//!
//! ```text
//! // Sender (any language):
//! OpValJsonValue { eq: "\"hello world\"" }
//! OpValJsonValue { eq: "42" }
//! OpValJsonValue { eq: "{\"k\":\"v\"}" }
//! ```
//!
//! # Integration with tonic
//!
//! A typical tonic service method looks like:
//!
//! ```ignore
//! use modql_proto::{ProtoFilterGroups, ProtoListOptions, ProtoConversionError};
//! use modql::filter::{FilterGroups, ListOptions};
//!
//! async fn list_tasks(
//!     &self,
//!     request: tonic::Request<ListTasksRequest>,
//! ) -> Result<tonic::Response<ListTasksResponse>, tonic::Status> {
//!     let req = request.into_inner();
//!
//!     let filter: FilterGroups = req
//!         .filter
//!         .map(FilterGroups::try_from)
//!         .transpose()
//!         .map_err(|e: ProtoConversionError| tonic::Status::invalid_argument(e.to_string()))?
//!         .unwrap_or_default();
//!
//!     let list_opts: ListOptions = req
//!         .list_options
//!         .map(ListOptions::from)
//!         .unwrap_or_default();
//!
//!     // … query the database using filter + list_opts …
//! }
//! ```

use modql::filter::{
	FilterGroups, FilterNode, FilterNodeOptions, ListOptions, OpVal, OpValBool, OpValFloat64, OpValInt32, OpValInt64,
	OpValString, OpValValue, OrderBy, OrderBys,
};

// =============================================================================
// Error type
// =============================================================================

/// Errors that can occur when converting proto messages into modql types.
#[derive(Debug)]
pub enum ProtoConversionError {
	/// A required oneof / optional field was absent in the proto message.
	MissingField(&'static str),
	/// A JSON-encoded value embedded in the proto message could not be parsed.
	InvalidJsonValue(serde_json::Error),
	/// A feature is not enabled.
	UnsupportedFeature(&'static str),
}

impl core::fmt::Display for ProtoConversionError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::MissingField(name) => write!(f, "missing required proto field: {name}"),
			Self::InvalidJsonValue(e) => write!(f, "invalid JSON value in proto message: {e}"),
			Self::UnsupportedFeature(f_name) => write!(f, "feature not enabled: {f_name}"),
		}
	}
}

impl std::error::Error for ProtoConversionError {}

// =============================================================================
// Helper list wrapper messages
// (proto3 does not allow `repeated` inside a `oneof`)
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StringList {
	#[prost(string, repeated, tag = "1")]
	pub values: Vec<String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Int64List {
	#[prost(int64, repeated, packed = "false", tag = "1")]
	pub values: Vec<i64>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Int32List {
	#[prost(int32, repeated, packed = "false", tag = "1")]
	pub values: Vec<i32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Float64List {
	#[prost(double, repeated, packed = "false", tag = "1")]
	pub values: Vec<f64>,
}

// =============================================================================
// Filter hierarchy
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoFilterGroups {
	#[prost(message, repeated, tag = "1")]
	pub groups: Vec<ProtoFilterGroup>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoFilterGroup {
	#[prost(message, repeated, tag = "1")]
	pub nodes: Vec<ProtoFilterNode>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoFilterNode {
	#[prost(string, tag = "1")]
	pub name: String,
	#[prost(string, optional, tag = "2")]
	pub rel: Option<String>,
	#[prost(message, repeated, tag = "3")]
	pub opvals: Vec<ProtoOpVal>,
	#[prost(message, optional, tag = "4")]
	pub options: Option<ProtoFilterNodeOptions>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoFilterNodeOptions {
	#[prost(string, optional, tag = "1")]
	pub cast_as: Option<String>,
	#[prost(string, optional, tag = "2")]
	pub cast_column_as: Option<String>,
}

// =============================================================================
// OpVal discriminated union
// =============================================================================

pub mod proto_op_val {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Value {
		#[prost(message, tag = "1")]
		String(super::ProtoOpValString),
		#[prost(message, tag = "2")]
		Int64(super::ProtoOpValInt64),
		#[prost(message, tag = "3")]
		Int32(super::ProtoOpValInt32),
		#[prost(message, tag = "4")]
		Float64(super::ProtoOpValFloat64),
		#[prost(message, tag = "5")]
		Bool(super::ProtoOpValBool),
		#[prost(message, tag = "6")]
		JsonValue(super::ProtoOpValJsonValue),
		#[prost(message, tag = "7")]
		Uuid(super::ProtoOpValUuid),
		#[prost(message, tag = "8")]
		Timestamp(super::ProtoOpValTimestamp),
	}
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpVal {
	#[prost(oneof = "proto_op_val::Value", tags = "1, 2, 3, 4, 5, 6, 7, 8")]
	pub value: Option<proto_op_val::Value>,
}

// =============================================================================
// String operators
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValString {
	#[prost(
		oneof = "proto_op_val_string::Op",
		tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30"
	)]
	pub op: Option<proto_op_val_string::Op>,
}

pub mod proto_op_val_string {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(string, tag = "1")]
		Eq(String),
		#[prost(string, tag = "2")]
		Not(String),
		#[prost(message, tag = "3")]
		In(super::StringList),
		#[prost(message, tag = "4")]
		NotIn(super::StringList),
		#[prost(string, tag = "5")]
		Lt(String),
		#[prost(string, tag = "6")]
		Lte(String),
		#[prost(string, tag = "7")]
		Gt(String),
		#[prost(string, tag = "8")]
		Gte(String),
		#[prost(string, tag = "9")]
		Contains(String),
		#[prost(string, tag = "10")]
		NotContains(String),
		#[prost(message, tag = "11")]
		ContainsAny(super::StringList),
		#[prost(message, tag = "12")]
		NotContainsAny(super::StringList),
		#[prost(message, tag = "13")]
		ContainsAll(super::StringList),
		#[prost(string, tag = "14")]
		StartsWith(String),
		#[prost(string, tag = "15")]
		NotStartsWith(String),
		#[prost(message, tag = "16")]
		StartsWithAny(super::StringList),
		#[prost(message, tag = "17")]
		NotStartsWithAny(super::StringList),
		#[prost(string, tag = "18")]
		EndsWith(String),
		#[prost(string, tag = "19")]
		NotEndsWith(String),
		#[prost(message, tag = "20")]
		EndsWithAny(super::StringList),
		#[prost(message, tag = "21")]
		NotEndsWithAny(super::StringList),
		#[prost(bool, tag = "22")]
		Empty(bool),
		#[prost(bool, tag = "23")]
		Null(bool),
		#[prost(string, tag = "24")]
		ContainsCi(String),
		#[prost(string, tag = "25")]
		NotContainsCi(String),
		#[prost(string, tag = "26")]
		StartsWithCi(String),
		#[prost(string, tag = "27")]
		NotStartsWithCi(String),
		#[prost(string, tag = "28")]
		EndsWithCi(String),
		#[prost(string, tag = "29")]
		NotEndsWithCi(String),
		#[prost(string, tag = "30")]
		Ilike(String),
	}
}

// =============================================================================
// Numeric operators
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValInt64 {
	#[prost(oneof = "proto_op_val_int64::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_int64::Op>,
}

pub mod proto_op_val_int64 {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(int64, tag = "1")]
		Eq(i64),
		#[prost(int64, tag = "2")]
		Not(i64),
		#[prost(message, tag = "3")]
		In(super::Int64List),
		#[prost(message, tag = "4")]
		NotIn(super::Int64List),
		#[prost(int64, tag = "5")]
		Lt(i64),
		#[prost(int64, tag = "6")]
		Lte(i64),
		#[prost(int64, tag = "7")]
		Gt(i64),
		#[prost(int64, tag = "8")]
		Gte(i64),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValInt32 {
	#[prost(oneof = "proto_op_val_int32::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_int32::Op>,
}

pub mod proto_op_val_int32 {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(int32, tag = "1")]
		Eq(i32),
		#[prost(int32, tag = "2")]
		Not(i32),
		#[prost(message, tag = "3")]
		In(super::Int32List),
		#[prost(message, tag = "4")]
		NotIn(super::Int32List),
		#[prost(int32, tag = "5")]
		Lt(i32),
		#[prost(int32, tag = "6")]
		Lte(i32),
		#[prost(int32, tag = "7")]
		Gt(i32),
		#[prost(int32, tag = "8")]
		Gte(i32),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValFloat64 {
	#[prost(oneof = "proto_op_val_float64::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_float64::Op>,
}

pub mod proto_op_val_float64 {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(double, tag = "1")]
		Eq(f64),
		#[prost(double, tag = "2")]
		Not(f64),
		#[prost(message, tag = "3")]
		In(super::Float64List),
		#[prost(message, tag = "4")]
		NotIn(super::Float64List),
		#[prost(double, tag = "5")]
		Lt(f64),
		#[prost(double, tag = "6")]
		Lte(f64),
		#[prost(double, tag = "7")]
		Gt(f64),
		#[prost(double, tag = "8")]
		Gte(f64),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

// =============================================================================
// Bool operators
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValBool {
	#[prost(oneof = "proto_op_val_bool::Op", tags = "1, 2, 3")]
	pub op: Option<proto_op_val_bool::Op>,
}

pub mod proto_op_val_bool {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(bool, tag = "1")]
		Eq(bool),
		#[prost(bool, tag = "2")]
		Not(bool),
		#[prost(bool, tag = "3")]
		Null(bool),
	}
}

// =============================================================================
// JSON value operators (values encoded as JSON strings)
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValJsonValue {
	#[prost(oneof = "proto_op_val_json_value::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_json_value::Op>,
}

pub mod proto_op_val_json_value {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		/// JSON-encoded scalar or object value.
		#[prost(string, tag = "1")]
		Eq(String),
		#[prost(string, tag = "2")]
		Not(String),
		/// List of JSON-encoded values.
		#[prost(message, tag = "3")]
		In(super::StringList),
		#[prost(message, tag = "4")]
		NotIn(super::StringList),
		#[prost(string, tag = "5")]
		Lt(String),
		#[prost(string, tag = "6")]
		Lte(String),
		#[prost(string, tag = "7")]
		Gt(String),
		#[prost(string, tag = "8")]
		Gte(String),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

// =============================================================================
// List options
// =============================================================================

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoListOptions {
	#[prost(int64, optional, tag = "1")]
	pub limit: Option<i64>,
	#[prost(int64, optional, tag = "2")]
	pub offset: Option<i64>,
	#[prost(message, optional, tag = "3")]
	pub order_bys: Option<ProtoOrderBys>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOrderBys {
	#[prost(message, repeated, tag = "1")]
	pub order_bys: Vec<ProtoOrderBy>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOrderBy {
	#[prost(oneof = "proto_order_by::Direction", tags = "1, 2")]
	pub direction: Option<proto_order_by::Direction>,
}

pub mod proto_order_by {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Direction {
		#[prost(string, tag = "1")]
		Asc(String),
		#[prost(string, tag = "2")]
		Desc(String),
	}
}

// =============================================================================
// TryFrom / From implementations  (proto → modql)
// =============================================================================

// -- FilterGroups -------------------------------------------------------------

impl TryFrom<ProtoFilterGroups> for FilterGroups {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoFilterGroups) -> Result<Self, Self::Error> {
		let groups_of_nodes: Result<Vec<Vec<FilterNode>>, _> = p
			.groups
			.into_iter()
			.map(|g| g.nodes.into_iter().map(FilterNode::try_from).collect::<Result<Vec<_>, _>>())
			.collect();
		Ok(FilterGroups::from(groups_of_nodes?))
	}
}

// -- FilterNode ---------------------------------------------------------------

impl TryFrom<ProtoFilterNode> for FilterNode {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoFilterNode) -> Result<Self, Self::Error> {
		let opvals: Vec<OpVal> = p.opvals.into_iter().map(OpVal::try_from).collect::<Result<_, _>>()?;
		let options = p.options.map(FilterNodeOptions::from).unwrap_or_default();
		let mut node = FilterNode::new_with_rel(p.rel, p.name, opvals);
		node.options = options;
		Ok(node)
	}
}

// -- FilterNodeOptions --------------------------------------------------------

impl From<ProtoFilterNodeOptions> for FilterNodeOptions {
	fn from(p: ProtoFilterNodeOptions) -> Self {
		FilterNodeOptions {
			cast_as: p.cast_as,
			cast_column_as: p.cast_column_as,
		}
	}
}

// -- OpVal --------------------------------------------------------------------

impl TryFrom<ProtoOpVal> for OpVal {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpVal) -> Result<Self, Self::Error> {
		match p.value {
			Some(proto_op_val::Value::String(v)) => Ok(OpVal::String(OpValString::try_from(v)?)),
			Some(proto_op_val::Value::Int64(v)) => Ok(OpVal::Int64(OpValInt64::try_from(v)?)),
			Some(proto_op_val::Value::Int32(v)) => Ok(OpVal::Int32(OpValInt32::try_from(v)?)),
			Some(proto_op_val::Value::Float64(v)) => Ok(OpVal::Float64(OpValFloat64::try_from(v)?)),
			Some(proto_op_val::Value::Bool(v)) => Ok(OpVal::Bool(OpValBool::try_from(v)?)),
			Some(proto_op_val::Value::Uuid(v)) => {
				#[cfg(feature = "uuid")]
				{
					use modql::filter::OpValUuid;
					return Ok(OpVal::Uuid(OpValUuid::try_from(v)?));
				}
				#[cfg(not(feature = "uuid"))]
				return Err(ProtoConversionError::UnsupportedFeature("uuid"));
			}
			Some(proto_op_val::Value::Timestamp(v)) => {
				#[cfg(feature = "chrono")]
				{
					use modql::filter::OpValTimestamp;
					return Ok(OpVal::Timestamp(OpValTimestamp::try_from(v)?));
				}
				#[cfg(not(feature = "chrono"))]
				return Err(ProtoConversionError::UnsupportedFeature("chrono"));
			}
			Some(proto_op_val::Value::JsonValue(v)) => Ok(OpVal::Value(OpValValue::try_from(v)?)),
			None => Err(ProtoConversionError::MissingField("OpVal.value")),
		}
	}
}

// -- OpValString --------------------------------------------------------------

impl TryFrom<ProtoOpValString> for OpValString {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValString) -> Result<Self, Self::Error> {
		use proto_op_val_string::Op;
		match p.op {
			Some(Op::Eq(v)) => Ok(OpValString::Eq(v)),
			Some(Op::Not(v)) => Ok(OpValString::Not(v)),
			Some(Op::In(l)) => Ok(OpValString::In(l.values)),
			Some(Op::NotIn(l)) => Ok(OpValString::NotIn(l.values)),
			Some(Op::Lt(v)) => Ok(OpValString::Lt(v)),
			Some(Op::Lte(v)) => Ok(OpValString::Lte(v)),
			Some(Op::Gt(v)) => Ok(OpValString::Gt(v)),
			Some(Op::Gte(v)) => Ok(OpValString::Gte(v)),
			Some(Op::Contains(v)) => Ok(OpValString::Contains(v)),
			Some(Op::NotContains(v)) => Ok(OpValString::NotContains(v)),
			Some(Op::ContainsAny(l)) => Ok(OpValString::ContainsAny(l.values)),
			Some(Op::NotContainsAny(l)) => Ok(OpValString::NotContainsAny(l.values)),
			Some(Op::ContainsAll(l)) => Ok(OpValString::ContainsAll(l.values)),
			Some(Op::StartsWith(v)) => Ok(OpValString::StartsWith(v)),
			Some(Op::NotStartsWith(v)) => Ok(OpValString::NotStartsWith(v)),
			Some(Op::StartsWithAny(l)) => Ok(OpValString::StartsWithAny(l.values)),
			Some(Op::NotStartsWithAny(l)) => Ok(OpValString::NotStartsWithAny(l.values)),
			Some(Op::EndsWith(v)) => Ok(OpValString::EndsWith(v)),
			Some(Op::NotEndsWith(v)) => Ok(OpValString::NotEndsWith(v)),
			Some(Op::EndsWithAny(l)) => Ok(OpValString::EndsWithAny(l.values)),
			Some(Op::NotEndsWithAny(l)) => Ok(OpValString::NotEndsWithAny(l.values)),
			Some(Op::Empty(v)) => Ok(OpValString::Empty(v)),
			Some(Op::Null(v)) => Ok(OpValString::Null(v)),
			Some(Op::ContainsCi(v)) => Ok(OpValString::ContainsCi(v)),
			Some(Op::NotContainsCi(v)) => Ok(OpValString::NotContainsCi(v)),
			Some(Op::StartsWithCi(v)) => Ok(OpValString::StartsWithCi(v)),
			Some(Op::NotStartsWithCi(v)) => Ok(OpValString::NotStartsWithCi(v)),
			Some(Op::EndsWithCi(v)) => Ok(OpValString::EndsWithCi(v)),
			Some(Op::NotEndsWithCi(v)) => Ok(OpValString::NotEndsWithCi(v)),
			Some(Op::Ilike(v)) => Ok(OpValString::Ilike(v)),
			None => Err(ProtoConversionError::MissingField("OpValString.op")),
		}
	}
}

// -- OpValInt64 ---------------------------------------------------------------

impl TryFrom<ProtoOpValInt64> for OpValInt64 {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValInt64) -> Result<Self, Self::Error> {
		use proto_op_val_int64::Op;
		match p.op {
			Some(Op::Eq(v)) => Ok(OpValInt64::Eq(v)),
			Some(Op::Not(v)) => Ok(OpValInt64::Not(v)),
			Some(Op::In(l)) => Ok(OpValInt64::In(l.values)),
			Some(Op::NotIn(l)) => Ok(OpValInt64::NotIn(l.values)),
			Some(Op::Lt(v)) => Ok(OpValInt64::Lt(v)),
			Some(Op::Lte(v)) => Ok(OpValInt64::Lte(v)),
			Some(Op::Gt(v)) => Ok(OpValInt64::Gt(v)),
			Some(Op::Gte(v)) => Ok(OpValInt64::Gte(v)),
			Some(Op::Null(v)) => Ok(OpValInt64::Null(v)),
			None => Err(ProtoConversionError::MissingField("OpValInt64.op")),
		}
	}
}

// -- OpValInt32 ---------------------------------------------------------------

impl TryFrom<ProtoOpValInt32> for OpValInt32 {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValInt32) -> Result<Self, Self::Error> {
		use proto_op_val_int32::Op;
		match p.op {
			Some(Op::Eq(v)) => Ok(OpValInt32::Eq(v)),
			Some(Op::Not(v)) => Ok(OpValInt32::Not(v)),
			Some(Op::In(l)) => Ok(OpValInt32::In(l.values)),
			Some(Op::NotIn(l)) => Ok(OpValInt32::NotIn(l.values)),
			Some(Op::Lt(v)) => Ok(OpValInt32::Lt(v)),
			Some(Op::Lte(v)) => Ok(OpValInt32::Lte(v)),
			Some(Op::Gt(v)) => Ok(OpValInt32::Gt(v)),
			Some(Op::Gte(v)) => Ok(OpValInt32::Gte(v)),
			Some(Op::Null(v)) => Ok(OpValInt32::Null(v)),
			None => Err(ProtoConversionError::MissingField("OpValInt32.op")),
		}
	}
}

// -- OpValFloat64 -------------------------------------------------------------

impl TryFrom<ProtoOpValFloat64> for OpValFloat64 {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValFloat64) -> Result<Self, Self::Error> {
		use proto_op_val_float64::Op;
		match p.op {
			Some(Op::Eq(v)) => Ok(OpValFloat64::Eq(v)),
			Some(Op::Not(v)) => Ok(OpValFloat64::Not(v)),
			Some(Op::In(l)) => Ok(OpValFloat64::In(l.values)),
			Some(Op::NotIn(l)) => Ok(OpValFloat64::NotIn(l.values)),
			Some(Op::Lt(v)) => Ok(OpValFloat64::Lt(v)),
			Some(Op::Lte(v)) => Ok(OpValFloat64::Lte(v)),
			Some(Op::Gt(v)) => Ok(OpValFloat64::Gt(v)),
			Some(Op::Gte(v)) => Ok(OpValFloat64::Gte(v)),
			Some(Op::Null(v)) => Ok(OpValFloat64::Null(v)),
			None => Err(ProtoConversionError::MissingField("OpValFloat64.op")),
		}
	}
}

// -- OpValBool ----------------------------------------------------------------

impl TryFrom<ProtoOpValBool> for OpValBool {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValBool) -> Result<Self, Self::Error> {
		use proto_op_val_bool::Op;
		match p.op {
			Some(Op::Eq(v)) => Ok(OpValBool::Eq(v)),
			Some(Op::Not(v)) => Ok(OpValBool::Not(v)),
			Some(Op::Null(v)) => Ok(OpValBool::Null(v)),
			None => Err(ProtoConversionError::MissingField("OpValBool.op")),
		}
	}
}

// -- OpValValue (JSON-encoded) ------------------------------------------------

impl TryFrom<ProtoOpValJsonValue> for OpValValue {
	type Error = ProtoConversionError;

	fn try_from(p: ProtoOpValJsonValue) -> Result<Self, Self::Error> {
		let parse =
			|s: String| serde_json::from_str::<serde_json::Value>(&s).map_err(ProtoConversionError::InvalidJsonValue);

		let parse_list = |l: StringList| -> Result<Vec<serde_json::Value>, ProtoConversionError> {
			l.values.into_iter().map(parse).collect()
		};

		use proto_op_val_json_value::Op;
		match p.op {
			Some(Op::Eq(s)) => Ok(OpValValue::Eq(parse(s)?)),
			Some(Op::Not(s)) => Ok(OpValValue::Not(parse(s)?)),
			Some(Op::In(l)) => Ok(OpValValue::In(parse_list(l)?)),
			Some(Op::NotIn(l)) => Ok(OpValValue::NotIn(parse_list(l)?)),
			Some(Op::Lt(s)) => Ok(OpValValue::Lt(parse(s)?)),
			Some(Op::Lte(s)) => Ok(OpValValue::Lte(parse(s)?)),
			Some(Op::Gt(s)) => Ok(OpValValue::Gt(parse(s)?)),
			Some(Op::Gte(s)) => Ok(OpValValue::Gte(parse(s)?)),
			Some(Op::Null(v)) => Ok(OpValValue::Null(v)),
			None => Err(ProtoConversionError::MissingField("OpValJsonValue.op")),
		}
	}
}

// -- ListOptions --------------------------------------------------------------

impl From<ProtoListOptions> for ListOptions {
	fn from(p: ProtoListOptions) -> Self {
		ListOptions {
			limit: p.limit,
			offset: p.offset,
			order_bys: p.order_bys.map(OrderBys::from),
		}
	}
}

// -- OrderBys -----------------------------------------------------------------

impl From<ProtoOrderBys> for OrderBys {
	fn from(p: ProtoOrderBys) -> Self {
		OrderBys::new(p.order_bys.into_iter().map(OrderBy::from).collect())
	}
}

// -- OrderBy ------------------------------------------------------------------

impl From<ProtoOrderBy> for OrderBy {
	fn from(p: ProtoOrderBy) -> Self {
		match p.direction {
			Some(proto_order_by::Direction::Asc(col)) => OrderBy::Asc(col),
			Some(proto_order_by::Direction::Desc(col)) => OrderBy::Desc(col),
			// Absent direction defaults to ascending with an empty column name
			// (callers should validate before sending).
			None => OrderBy::Asc(String::new()),
		}
	}
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValUuid {
	#[prost(oneof = "proto_op_val_uuid::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_uuid::Op>,
}

pub mod proto_op_val_uuid {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(string, tag = "1")]
		Eq(String),
		#[prost(string, tag = "2")]
		Not(String),
		#[prost(message, tag = "3")]
		In(super::StringList),
		#[prost(message, tag = "4")]
		NotIn(super::StringList),
		#[prost(string, tag = "5")]
		Lt(String),
		#[prost(string, tag = "6")]
		Lte(String),
		#[prost(string, tag = "7")]
		Gt(String),
		#[prost(string, tag = "8")]
		Gte(String),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TimestampList {
	#[prost(message, repeated, tag = "1")]
	pub values: ::prost::alloc::vec::Vec<prost_types::Timestamp>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProtoOpValTimestamp {
	#[prost(oneof = "proto_op_val_timestamp::Op", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9")]
	pub op: Option<proto_op_val_timestamp::Op>,
}

pub mod proto_op_val_timestamp {
	#[derive(Clone, PartialEq, ::prost::Oneof)]
	pub enum Op {
		#[prost(message, tag = "1")]
		Eq(prost_types::Timestamp),
		#[prost(message, tag = "2")]
		Not(prost_types::Timestamp),
		#[prost(message, tag = "3")]
		In(super::TimestampList),
		#[prost(message, tag = "4")]
		NotIn(super::TimestampList),
		#[prost(message, tag = "5")]
		Lt(prost_types::Timestamp),
		#[prost(message, tag = "6")]
		Lte(prost_types::Timestamp),
		#[prost(message, tag = "7")]
		Gt(prost_types::Timestamp),
		#[prost(message, tag = "8")]
		Gte(prost_types::Timestamp),
		#[prost(bool, tag = "9")]
		Null(bool),
	}
}

// -- OpValUuid --------------------------------------------------------------

#[cfg(feature = "uuid")]
pub mod uuid_impl {
	use crate::proto_op_val_uuid::Op;
	use crate::{ProtoConversionError, ProtoOpValUuid};
	use modql::filter::OpValUuid;
	use uuid::Uuid;

	fn parse_uuid(v: &str) -> Result<Uuid, ProtoConversionError> {
		data_encoding::BASE64URL_NOPAD
			.decode(v.as_bytes())
			.map_err(|_| ProtoConversionError::MissingField("Invalid UUID"))
			.and_then(|decoded| {
				Uuid::from_slice(&decoded).map_err(|_| ProtoConversionError::MissingField("Invalid UUID"))
			})
	}

	impl TryFrom<ProtoOpValUuid> for OpValUuid {
		type Error = ProtoConversionError;

		fn try_from(p: ProtoOpValUuid) -> Result<Self, Self::Error> {
			match p.op {
				Some(Op::Eq(v)) => Ok(OpValUuid::Eq(parse_uuid(&v)?)),
				Some(Op::Not(v)) => Ok(OpValUuid::Not(parse_uuid(&v)?)),
				Some(Op::In(l)) => Ok(OpValUuid::In(
					l.values.into_iter().map(|v| parse_uuid(&v)).collect::<Result<Vec<_>, _>>()?,
				)),
				Some(Op::NotIn(l)) => Ok(OpValUuid::NotIn(
					l.values.into_iter().map(|v| parse_uuid(&v)).collect::<Result<Vec<_>, _>>()?,
				)),
				Some(Op::Lt(v)) => Ok(OpValUuid::Lt(parse_uuid(&v)?)),
				Some(Op::Lte(v)) => Ok(OpValUuid::Lte(parse_uuid(&v)?)),
				Some(Op::Gt(v)) => Ok(OpValUuid::Gt(parse_uuid(&v)?)),
				Some(Op::Gte(v)) => Ok(OpValUuid::Gte(parse_uuid(&v)?)),
				Some(Op::Null(v)) => Ok(OpValUuid::Null(v)),
				None => Err(ProtoConversionError::MissingField("OpValUuid.op")),
			}
		}
	}
}

// -- OpValTimestamp -----------------------------------------------------------

#[cfg(feature = "chrono")]
pub mod chrono_impl {
	use crate::{ProtoConversionError, ProtoOpValTimestamp, TimestampList};
	use modql::filter::OpValTimestamp;

	impl TryFrom<ProtoOpValTimestamp> for OpValTimestamp {
		type Error = ProtoConversionError;

		fn try_from(p: ProtoOpValTimestamp) -> Result<Self, Self::Error> {
			use crate::proto_op_val_timestamp::Op;
			use chrono::{DateTime, Utc};

			fn to_datetime(t: prost_types::Timestamp) -> Result<DateTime<Utc>, ProtoConversionError> {
				chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
					.ok_or(ProtoConversionError::MissingField("Invalid Timestamp"))
			}

			match p.op {
				Some(Op::Eq(v)) => Ok(OpValTimestamp::Eq(to_datetime(v)?)),
				Some(Op::Not(v)) => Ok(OpValTimestamp::Not(to_datetime(v)?)),
				Some(Op::In(l)) => Ok(OpValTimestamp::In(
					l.values.into_iter().map(to_datetime).collect::<Result<Vec<_>, _>>()?,
				)),
				Some(Op::NotIn(l)) => Ok(OpValTimestamp::NotIn(
					l.values.into_iter().map(to_datetime).collect::<Result<Vec<_>, _>>()?,
				)),
				Some(Op::Lt(v)) => Ok(OpValTimestamp::Lt(to_datetime(v)?)),
				Some(Op::Lte(v)) => Ok(OpValTimestamp::Lte(to_datetime(v)?)),
				Some(Op::Gt(v)) => Ok(OpValTimestamp::Gt(to_datetime(v)?)),
				Some(Op::Gte(v)) => Ok(OpValTimestamp::Gte(to_datetime(v)?)),
				Some(Op::Null(v)) => Ok(OpValTimestamp::Null(v)),
				None => Err(ProtoConversionError::MissingField("OpValTimestamp.op")),
			}
		}
	}

	#[cfg(feature = "chrono")]
	impl From<OpValTimestamp> for ProtoOpValTimestamp {
		fn from(op: OpValTimestamp) -> Self {
			use crate::proto_op_val_timestamp::Op;
			use chrono::{DateTime, Timelike, Utc};

			fn to_proto_timestamp(dt: DateTime<Utc>) -> prost_types::Timestamp {
				prost_types::Timestamp {
					seconds: dt.timestamp(),
					nanos: dt.nanosecond() as i32,
				}
			}

			let op_enum = match op {
				OpValTimestamp::Eq(dt) => Op::Eq(to_proto_timestamp(dt)),
				OpValTimestamp::Not(dt) => Op::Not(to_proto_timestamp(dt)),
				OpValTimestamp::In(dts) => Op::In(TimestampList {
					values: dts.into_iter().map(to_proto_timestamp).collect(),
				}),
				OpValTimestamp::NotIn(dts) => Op::NotIn(TimestampList {
					values: dts.into_iter().map(to_proto_timestamp).collect(),
				}),
				OpValTimestamp::Lt(dt) => Op::Lt(to_proto_timestamp(dt)),
				OpValTimestamp::Lte(dt) => Op::Lte(to_proto_timestamp(dt)),
				OpValTimestamp::Gt(dt) => Op::Gt(to_proto_timestamp(dt)),
				OpValTimestamp::Gte(dt) => Op::Gte(to_proto_timestamp(dt)),
				OpValTimestamp::Null(v) => Op::Null(v),
			};
			ProtoOpValTimestamp { op: Some(op_enum) }
		}
	}
}
