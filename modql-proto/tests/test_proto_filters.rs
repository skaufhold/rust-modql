//! Integration tests for the modql-proto crate.
//!
//! These tests verify that:
//! 1. Proto messages can be converted directly into modql types.
//! 2. The prost wire-encoding roundtrip (encode → bytes → decode → convert) works correctly.
//! 3. `ProtoConversionError` is returned for malformed messages.

use modql::filter::{
    FilterGroups, ListOptions, OpVal, OpValBool, OpValFloat64, OpValInt32, OpValInt64, OpValString,
    OpValValue, OrderBy
};
use modql_proto::{
    // error
    ProtoConversionError,
    // filter hierarchy
    ProtoFilterGroup,
    ProtoFilterGroups,
    ProtoFilterNode,
    ProtoFilterNodeOptions,
    // list options
    ProtoListOptions,
    ProtoOpVal,
    // op val types
    ProtoOpValBool,
    ProtoOpValFloat64,
    ProtoOpValInt32,
    ProtoOpValInt64,
    ProtoOpValJsonValue,
    ProtoOpValString,
    ProtoOpValTimestamp,
    ProtoOrderBy,
    ProtoOrderBys,
    // list helpers
    Float64List,
    Int32List,
    Int64List,
    StringList,
    // inner oneof modules
    proto_op_val,
    proto_op_val_bool,
    proto_op_val_float64,
    proto_op_val_int32,
    proto_op_val_int64,
    proto_op_val_json_value,
    proto_op_val_string,
    proto_op_val_timestamp,
    proto_order_by,
};
use prost::Message as _;

// =============================================================================
// Helpers
// =============================================================================

/// Build a single-variant `ProtoOpVal` wrapping a `ProtoOpValString`.
fn string_opval(op: proto_op_val_string::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::String(ProtoOpValString { op: Some(op) })),
    }
}

/// Build a single-variant `ProtoOpVal` wrapping a `ProtoOpValInt64`.
fn i64_opval(op: proto_op_val_int64::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::Int64(ProtoOpValInt64 { op: Some(op) })),
    }
}

fn i32_opval(op: proto_op_val_int32::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::Int32(ProtoOpValInt32 { op: Some(op) })),
    }
}

fn f64_opval(op: proto_op_val_float64::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::Float64(ProtoOpValFloat64 { op: Some(op) })),
    }
}

fn bool_opval(op: proto_op_val_bool::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::Bool(ProtoOpValBool { op: Some(op) })),
    }
}

fn json_opval(op: proto_op_val_json_value::Op) -> ProtoOpVal {
    ProtoOpVal {
        value: Some(proto_op_val::Value::JsonValue(ProtoOpValJsonValue { op: Some(op) })),
    }
}

/// Wrap a single `ProtoOpVal` into a minimal `ProtoFilterGroups`.
fn single_node_groups(name: &str, opval: ProtoOpVal) -> ProtoFilterGroups {
    ProtoFilterGroups {
        groups: vec![ProtoFilterGroup {
            nodes: vec![ProtoFilterNode {
                name: name.to_string(),
                rel: None,
                opvals: vec![opval],
                options: None,
            }],
        }],
    }
}

/// Pull the first OpVal out of a `FilterGroups` (panics if absent).
fn first_opval(fg: FilterGroups) -> OpVal {
    fg.into_vec()
        .into_iter()
        .next()
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .opvals
        .into_iter()
        .next()
        .unwrap()
}

// =============================================================================
// OpValString — single-value operators
// =============================================================================

#[test]
fn test_string_eq() {
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::Eq("Alice".into())),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::Eq(s)) if s == "Alice"));
}

#[test]
fn test_string_not() {
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::Not("Bob".into())),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::Not(s)) if s == "Bob"));
}

#[test]
fn test_string_lt_lte_gt_gte() {
    let cases = [
        (proto_op_val_string::Op::Lt("C".into()), "Lt"),
        (proto_op_val_string::Op::Lte("C".into()), "Lte"),
        (proto_op_val_string::Op::Gt("J".into()), "Gt"),
        (proto_op_val_string::Op::Gte("J".into()), "Gte"),
    ];
    for (op, label) in cases {
        let fg = FilterGroups::try_from(single_node_groups("name", string_opval(op))).unwrap();
        let ov = first_opval(fg);
        assert!(
            matches!(&ov, OpVal::String(_)),
            "{label}: expected OpVal::String, got {ov:?}"
        );
    }
}

#[test]
fn test_string_contains_and_variants() {
    let ops: Vec<proto_op_val_string::Op> = vec![
        proto_op_val_string::Op::Contains("foo".into()),
        proto_op_val_string::Op::NotContains("foo".into()),
        proto_op_val_string::Op::StartsWith("pre".into()),
        proto_op_val_string::Op::NotStartsWith("pre".into()),
        proto_op_val_string::Op::EndsWith("suf".into()),
        proto_op_val_string::Op::NotEndsWith("suf".into()),
        proto_op_val_string::Op::ContainsCi("FOO".into()),
        proto_op_val_string::Op::NotContainsCi("FOO".into()),
        proto_op_val_string::Op::StartsWithCi("PRE".into()),
        proto_op_val_string::Op::NotStartsWithCi("PRE".into()),
        proto_op_val_string::Op::EndsWithCi("SUF".into()),
        proto_op_val_string::Op::NotEndsWithCi("SUF".into()),
        proto_op_val_string::Op::Ilike("pat".into()),
    ];
    for op in ops {
        let fg = FilterGroups::try_from(single_node_groups("title", string_opval(op))).unwrap();
        assert!(matches!(first_opval(fg), OpVal::String(_)));
    }
}

#[test]
fn test_string_empty_and_null() {
    let fg = FilterGroups::try_from(single_node_groups(
        "bio",
        string_opval(proto_op_val_string::Op::Empty(true)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::Empty(true))));

    let fg = FilterGroups::try_from(single_node_groups(
        "bio",
        string_opval(proto_op_val_string::Op::Null(false)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::Null(false))));
}

// =============================================================================
// OpValString — list-value operators
// =============================================================================

#[test]
fn test_string_in_list() {
    let list = StringList {
        values: vec!["Alice".into(), "Bob".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::In(list)),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::String(OpValString::In(v)) if v == vec!["Alice", "Bob"]
    ));
}

#[test]
fn test_string_not_in_list() {
    let list = StringList { values: vec!["X".into()] };
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::NotIn(list)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::NotIn(_))));
}

#[test]
fn test_string_contains_any_and_all() {
    let list_any = StringList {
        values: vec!["hello".into(), "world".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "title",
        string_opval(proto_op_val_string::Op::ContainsAny(list_any)),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::String(OpValString::ContainsAny(v)) if v.len() == 2
    ));

    let list_all = StringList {
        values: vec!["hello".into(), "world".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "title",
        string_opval(proto_op_val_string::Op::ContainsAll(list_all)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::ContainsAll(_))));
}

#[test]
fn test_string_starts_with_any() {
    let list = StringList {
        values: vec!["Mr".into(), "Dr".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::StartsWithAny(list)),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::String(OpValString::StartsWithAny(v)) if v == vec!["Mr", "Dr"]
    ));
}

#[test]
fn test_string_ends_with_any() {
    let list = StringList {
        values: vec!["Jr".into(), "Sr".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "name",
        string_opval(proto_op_val_string::Op::EndsWithAny(list)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::String(OpValString::EndsWithAny(_))));
}

// =============================================================================
// OpValInt64
// =============================================================================

#[test]
fn test_int64_eq() {
    let fg = FilterGroups::try_from(single_node_groups(
        "id",
        i64_opval(proto_op_val_int64::Op::Eq(42)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Int64(OpValInt64::Eq(42))));
}

#[test]
fn test_int64_comparison_ops() {
    let cases = [
        (proto_op_val_int64::Op::Not(1), "Not"),
        (proto_op_val_int64::Op::Lt(10), "Lt"),
        (proto_op_val_int64::Op::Lte(10), "Lte"),
        (proto_op_val_int64::Op::Gt(10), "Gt"),
        (proto_op_val_int64::Op::Gte(10), "Gte"),
        (proto_op_val_int64::Op::Null(true), "Null"),
    ];
    for (op, label) in cases {
        let fg = FilterGroups::try_from(single_node_groups("id", i64_opval(op))).unwrap();
        assert!(
            matches!(&first_opval(fg), OpVal::Int64(_)),
            "{label}: expected OpVal::Int64"
        );
    }
}

#[test]
fn test_int64_in_list() {
    let list = Int64List { values: vec![1, 2, 3] };
    let fg = FilterGroups::try_from(single_node_groups(
        "id",
        i64_opval(proto_op_val_int64::Op::In(list)),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::Int64(OpValInt64::In(v)) if v == vec![1i64, 2, 3]
    ));
}

#[test]
fn test_int64_not_in_list() {
    let list = Int64List { values: vec![99, 100] };
    let fg = FilterGroups::try_from(single_node_groups(
        "id",
        i64_opval(proto_op_val_int64::Op::NotIn(list)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Int64(OpValInt64::NotIn(_))));
}

// =============================================================================
// OpValInt32
// =============================================================================

#[test]
fn test_int32_eq() {
    let fg = FilterGroups::try_from(single_node_groups(
        "count",
        i32_opval(proto_op_val_int32::Op::Eq(7)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Int32(OpValInt32::Eq(7))));
}

#[test]
fn test_int32_in_list() {
    let list = Int32List { values: vec![1, 2] };
    let fg = FilterGroups::try_from(single_node_groups(
        "count",
        i32_opval(proto_op_val_int32::Op::In(list)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Int32(OpValInt32::In(_))));
}

// =============================================================================
// OpValFloat64
// =============================================================================

#[test]
fn test_float64_eq() {
    let fg = FilterGroups::try_from(single_node_groups(
        "price",
        f64_opval(proto_op_val_float64::Op::Eq(9.99)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Float64(OpValFloat64::Eq(v)) if (v - 9.99).abs() < f64::EPSILON));
}

#[test]
fn test_float64_in_list() {
    let list = Float64List { values: vec![1.1, 2.2] };
    let fg = FilterGroups::try_from(single_node_groups(
        "price",
        f64_opval(proto_op_val_float64::Op::In(list)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Float64(OpValFloat64::In(_))));
}

#[test]
fn test_float64_null() {
    let fg = FilterGroups::try_from(single_node_groups(
        "price",
        f64_opval(proto_op_val_float64::Op::Null(true)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Float64(OpValFloat64::Null(true))));
}

// =============================================================================
// OpValBool
// =============================================================================

#[test]
fn test_bool_eq() {
    let fg = FilterGroups::try_from(single_node_groups(
        "active",
        bool_opval(proto_op_val_bool::Op::Eq(true)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Bool(OpValBool::Eq(true))));
}

#[test]
fn test_bool_not() {
    let fg = FilterGroups::try_from(single_node_groups(
        "active",
        bool_opval(proto_op_val_bool::Op::Not(false)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Bool(OpValBool::Not(false))));
}

#[test]
fn test_bool_null() {
    let fg = FilterGroups::try_from(single_node_groups(
        "active",
        bool_opval(proto_op_val_bool::Op::Null(true)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Bool(OpValBool::Null(true))));
}

// =============================================================================
// OpValJsonValue
// =============================================================================

#[test]
fn test_json_value_eq_scalar() {
    let fg = FilterGroups::try_from(single_node_groups(
        "meta",
        json_opval(proto_op_val_json_value::Op::Eq("42".into())),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::Value(OpValValue::Eq(v)) if v == serde_json::json!(42)
    ));
}

#[test]
fn test_json_value_eq_string_json() {
    let fg = FilterGroups::try_from(single_node_groups(
        "meta",
        json_opval(proto_op_val_json_value::Op::Eq(r#""hello""#.into())),
    ))
    .unwrap();
    assert!(matches!(
        first_opval(fg),
        OpVal::Value(OpValValue::Eq(v)) if v == serde_json::json!("hello")
    ));
}

#[test]
fn test_json_value_null() {
    let fg = FilterGroups::try_from(single_node_groups(
        "meta",
        json_opval(proto_op_val_json_value::Op::Null(true)),
    ))
    .unwrap();
    assert!(matches!(first_opval(fg), OpVal::Value(OpValValue::Null(true))));
}

#[test]
fn test_json_value_in_list() {
    let list = StringList {
        values: vec!["1".into(), "2".into(), "3".into()],
    };
    let fg = FilterGroups::try_from(single_node_groups(
        "meta",
        json_opval(proto_op_val_json_value::Op::In(list)),
    ))
    .unwrap();
    let OpVal::Value(OpValValue::In(vals)) = first_opval(fg) else {
        panic!("expected OpVal::Value(OpValValue::In(..))");
    };
    assert_eq!(vals, vec![serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)]);
}

#[test]
fn test_json_value_invalid_json_returns_error() {
    let proto = ProtoOpValJsonValue {
        op: Some(proto_op_val_json_value::Op::Eq("not-valid-json{{".into())),
    };
    let result = OpValValue::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::InvalidJsonValue(_))));
}

// =============================================================================
// FilterNode — rel, options
// =============================================================================

#[test]
fn test_filter_node_with_rel() {
    let proto = ProtoFilterGroups {
        groups: vec![ProtoFilterGroup {
            nodes: vec![ProtoFilterNode {
                name: "title".into(),
                rel: Some("task".into()),
                opvals: vec![string_opval(proto_op_val_string::Op::Eq("hello".into()))],
                options: None,
            }],
        }],
    };
    let fg = FilterGroups::try_from(proto).unwrap();
    let node = fg.into_vec().into_iter().next().unwrap().into_iter().next().unwrap();
    assert_eq!(node.rel.as_deref(), Some("task"));
    assert_eq!(node.name, "title");
}

#[test]
fn test_filter_node_options_preserved() {
    let proto = ProtoFilterGroups {
        groups: vec![ProtoFilterGroup {
            nodes: vec![ProtoFilterNode {
                name: "value".into(),
                rel: None,
                opvals: vec![i64_opval(proto_op_val_int64::Op::Eq(1))],
                options: Some(ProtoFilterNodeOptions {
                    cast_as: Some("bigint".into()),
                    cast_column_as: Some("text".into()),
                }),
            }],
        }],
    };
    let fg = FilterGroups::try_from(proto).unwrap();
    let node = fg.into_vec().into_iter().next().unwrap().into_iter().next().unwrap();
    assert_eq!(node.options.cast_as.as_deref(), Some("bigint"));
    assert_eq!(node.options.cast_column_as.as_deref(), Some("text"));
}

// =============================================================================
// FilterGroups — multiple groups and nodes (OR / AND semantics)
// =============================================================================

#[test]
fn test_filter_groups_multi_group() {
    let proto = ProtoFilterGroups {
        groups: vec![
            ProtoFilterGroup {
                nodes: vec![
                    ProtoFilterNode {
                        name: "title".into(),
                        rel: None,
                        opvals: vec![string_opval(proto_op_val_string::Op::Eq("Hello".into()))],
                        options: None,
                    },
                    ProtoFilterNode {
                        name: "done".into(),
                        rel: None,
                        opvals: vec![bool_opval(proto_op_val_bool::Op::Eq(true))],
                        options: None,
                    },
                ],
            },
            ProtoFilterGroup {
                nodes: vec![ProtoFilterNode {
                    name: "id".into(),
                    rel: None,
                    opvals: vec![i64_opval(proto_op_val_int64::Op::Gt(100))],
                    options: None,
                }],
            },
        ],
    };

    let fg = FilterGroups::try_from(proto).unwrap();
    let groups = fg.into_vec();
    assert_eq!(groups.len(), 2, "expected 2 groups");
    assert_eq!(groups[0].nodes().len(), 2, "group 0 should have 2 nodes");
    assert_eq!(groups[1].nodes().len(), 1, "group 1 should have 1 node");
}

#[test]
fn test_filter_node_multiple_opvals() {
    let proto = ProtoFilterGroups {
        groups: vec![ProtoFilterGroup {
            nodes: vec![ProtoFilterNode {
                name: "id".into(),
                rel: None,
                opvals: vec![
                    i64_opval(proto_op_val_int64::Op::Gt(10)),
                    i64_opval(proto_op_val_int64::Op::Lt(50)),
                ],
                options: None,
            }],
        }],
    };

    let fg = FilterGroups::try_from(proto).unwrap();
    let node = fg.into_vec().into_iter().next().unwrap().into_iter().next().unwrap();
    assert_eq!(node.opvals.len(), 2);
    assert!(matches!(node.opvals[0], OpVal::Int64(OpValInt64::Gt(10))));
    assert!(matches!(node.opvals[1], OpVal::Int64(OpValInt64::Lt(50))));
}

// =============================================================================
// ListOptions / OrderBys / OrderBy
// =============================================================================

#[test]
fn test_list_options_limit_offset() {
    let proto = ProtoListOptions {
        limit: Some(25),
        offset: Some(50),
        order_bys: None,
    };
    let lo = ListOptions::from(proto);
    assert_eq!(lo.limit, Some(25));
    assert_eq!(lo.offset, Some(50));
    assert!(lo.order_bys.is_none());
}

#[test]
fn test_list_options_order_bys_asc_desc() {
    let proto = ProtoListOptions {
        limit: None,
        offset: None,
        order_bys: Some(ProtoOrderBys {
            order_bys: vec![
                ProtoOrderBy {
                    direction: Some(proto_order_by::Direction::Asc("name".into())),
                },
                ProtoOrderBy {
                    direction: Some(proto_order_by::Direction::Desc("created_at".into())),
                },
            ],
        }),
    };
    let lo = ListOptions::from(proto);
    let order_bys: Vec<OrderBy> = lo.order_bys.unwrap().into_iter().collect();
    assert_eq!(order_bys.len(), 2);
    assert!(matches!(&order_bys[0], OrderBy::Asc(col) if col == "name"));
    assert!(matches!(&order_bys[1], OrderBy::Desc(col) if col == "created_at"));
}

#[test]
fn test_order_by_absent_direction_defaults_to_asc() {
    let ob = OrderBy::from(ProtoOrderBy { direction: None });
    assert!(matches!(ob, OrderBy::Asc(col) if col.is_empty()));
}

// =============================================================================
// Error cases
// =============================================================================

#[test]
fn test_missing_opval_value_returns_error() {
    let proto = ProtoOpVal { value: None };
    let result = OpVal::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::MissingField("OpVal.value"))));
}

#[test]
fn test_missing_op_val_string_op_returns_error() {
    let proto = ProtoOpValString { op: None };
    let result = OpValString::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::MissingField("OpValString.op"))));
}

#[test]
fn test_missing_op_val_int64_op_returns_error() {
    let proto = ProtoOpValInt64 { op: None };
    let result = OpValInt64::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::MissingField("OpValInt64.op"))));
}

#[test]
fn test_missing_op_val_bool_op_returns_error() {
    let proto = ProtoOpValBool { op: None };
    let result = OpValBool::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::MissingField("OpValBool.op"))));
}

#[test]
fn test_missing_op_val_json_value_op_returns_error() {
    let proto = ProtoOpValJsonValue { op: None };
    let result = OpValValue::try_from(proto);
    assert!(matches!(result, Err(ProtoConversionError::MissingField("OpValJsonValue.op"))));
}

// =============================================================================
// Wire-format roundtrip (encode → bytes → decode → convert)
// =============================================================================

#[test]
fn test_wire_roundtrip_filter_groups() {
    let original = ProtoFilterGroups {
        groups: vec![
            ProtoFilterGroup {
                nodes: vec![
                    ProtoFilterNode {
                        name: "title".into(),
                        rel: None,
                        opvals: vec![string_opval(proto_op_val_string::Op::Contains("hello".into()))],
                        options: None,
                    },
                    ProtoFilterNode {
                        name: "price".into(),
                        rel: None,
                        opvals: vec![f64_opval(proto_op_val_float64::Op::Gt(0.0))],
                        options: None,
                    },
                ],
            },
            ProtoFilterGroup {
                nodes: vec![ProtoFilterNode {
                    name: "id".into(),
                    rel: Some("item".into()),
                    opvals: vec![
                        i64_opval(proto_op_val_int64::Op::In(Int64List { values: vec![1, 2, 3] })),
                    ],
                    options: Some(ProtoFilterNodeOptions {
                        cast_as: Some("bigint".into()),
                        cast_column_as: None,
                    }),
                }],
            },
        ],
    };

    let bytes = original.encode_to_vec();
    let decoded = ProtoFilterGroups::decode(bytes.as_slice()).expect("proto decode failed");
    let fg = FilterGroups::try_from(decoded).expect("proto conversion failed");
    let groups = fg.into_vec();

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].nodes().len(), 2);
    assert_eq!(groups[0].nodes()[0].name, "title");
    assert!(matches!(&groups[0].nodes()[0].opvals[0], OpVal::String(OpValString::Contains(s)) if s == "hello"));

    assert_eq!(groups[1].nodes().len(), 1);
    let id_node = &groups[1].nodes()[0];
    assert_eq!(id_node.rel.as_deref(), Some("item"));
    assert_eq!(id_node.options.cast_as.as_deref(), Some("bigint"));
    assert!(matches!(&id_node.opvals[0], OpVal::Int64(OpValInt64::In(v)) if *v == vec![1i64, 2, 3]));
}

#[test]
fn test_wire_roundtrip_list_options() {
    let original = ProtoListOptions {
        limit: Some(10),
        offset: Some(20),
        order_bys: Some(ProtoOrderBys {
            order_bys: vec![
                ProtoOrderBy {
                    direction: Some(proto_order_by::Direction::Asc("name".into())),
                },
                ProtoOrderBy {
                    direction: Some(proto_order_by::Direction::Desc("updated_at".into())),
                },
            ],
        }),
    };

    let bytes = original.encode_to_vec();
    let decoded = ProtoListOptions::decode(bytes.as_slice()).expect("proto decode failed");
    let lo = ListOptions::from(decoded);

    assert_eq!(lo.limit, Some(10));
    assert_eq!(lo.offset, Some(20));

    let order_bys: Vec<OrderBy> = lo.order_bys.unwrap().into_iter().collect();
    assert_eq!(order_bys.len(), 2);
    assert!(matches!(&order_bys[0], OrderBy::Asc(col) if col == "name"));
    assert!(matches!(&order_bys[1], OrderBy::Desc(col) if col == "updated_at"));
}

#[test]
fn test_wire_roundtrip_all_bool_ops() {
    for op in [
        proto_op_val_bool::Op::Eq(true),
        proto_op_val_bool::Op::Not(false),
        proto_op_val_bool::Op::Null(true),
    ] {
        let original = single_node_groups("active", bool_opval(op));
        let bytes = original.encode_to_vec();
        let decoded = ProtoFilterGroups::decode(bytes.as_slice()).unwrap();
        let fg = FilterGroups::try_from(decoded).unwrap();
        assert!(matches!(first_opval(fg), OpVal::Bool(_)));
    }
}

#[test]
fn test_wire_roundtrip_json_value_object() {
    let json_str = r#"{"key":"value","num":123}"#;
    let original = single_node_groups(
        "payload",
        json_opval(proto_op_val_json_value::Op::Eq(json_str.into())),
    );

    let bytes = original.encode_to_vec();
    let decoded = ProtoFilterGroups::decode(bytes.as_slice()).unwrap();
    let fg = FilterGroups::try_from(decoded).unwrap();

    let OpVal::Value(OpValValue::Eq(val)) = first_opval(fg) else {
        panic!("expected OpVal::Value(OpValValue::Eq(..))");
    };
    assert_eq!(val["key"], serde_json::json!("value"));
    assert_eq!(val["num"], serde_json::json!(123));
}

#[test]
fn test_empty_filter_groups() {
    let proto = ProtoFilterGroups { groups: vec![] };
    let fg = FilterGroups::try_from(proto).unwrap();
    assert!(fg.groups().is_empty());
}

#[test]
fn test_empty_order_bys() {
    let proto = ProtoListOptions {
        limit: None,
        offset: None,
        order_bys: Some(ProtoOrderBys { order_bys: vec![] }),
    };
    let lo = ListOptions::from(proto);
    let obs: Vec<_> = lo.order_bys.unwrap().into_iter().collect();
    assert!(obs.is_empty());
}

#[cfg(feature = "chrono")]
#[test]
fn test_wire_roundtrip_timestamp_ops() {
    use modql_proto::proto_op_val_timestamp;
    use chrono::{DateTime, Utc, Timelike};

    let now = Utc::now();
    let ts = prost_types::Timestamp {
        seconds: now.timestamp(),
        nanos: now.nanosecond() as i32,
    };

    let original = single_node_groups("created_at", ProtoOpVal {
        value: Some(proto_op_val::Value::Timestamp(ProtoOpValTimestamp {
            op: Some(proto_op_val_timestamp::Op::Eq(ts)),
        })),
    });

    let bytes = original.encode_to_vec();
    let decoded = ProtoFilterGroups::decode(bytes.as_slice()).unwrap();
    let fg = FilterGroups::try_from(decoded).unwrap();
    
    let opval = first_opval(fg);
    assert!(matches!(opval, OpVal::Timestamp(_)));
}
