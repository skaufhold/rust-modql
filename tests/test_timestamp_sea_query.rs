#![cfg(all(feature = "with-sea-query", feature = "chrono"))]

use modql::filter::{FilterNode, OpVal, OpValTimestamp, OpValsTimestamp, FilterNodes};
use sea_query::{ColumnRef, ColumnName, ExprTrait, SimpleExpr};
use chrono::{DateTime, Utc};

#[derive(FilterNodes, Default)]
pub struct TimeFilter {
    time: Option<OpValsTimestamp>,
}

#[test]
fn test_timestamp_sea_query_integration() -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let node: FilterNode = ("time", OpValTimestamp::Eq(now)).into();
    
    // We want to verify that into_sea_cond_expr_list works for Timestamp
    let conditions = node.into_sea_cond_expr_list()?;
    
    assert_eq!(conditions.len(), 1);
    
    // Check if the condition is correctly generated (e.g. check for "ctime")
    let cond = &conditions[0];
    let query = sea_query::Query::select().cond_where(cond.clone()).to_owned();
    let (sql, _) = query.build(sea_query::PostgresQueryBuilder);
    
    assert!(sql.contains(r#""time" = $1"#));
    
    // Test other variants
    let opval_in = OpValTimestamp::In(vec![now, now]);
    let node_in: FilterNode = ("time", opval_in).into();
    
    let conditions_in = node_in.into_sea_cond_expr_list()?;
    let query_in = sea_query::Query::select().cond_where(conditions_in[0].clone()).to_owned();
    let (sql_in, _) = query_in.build(sea_query::PostgresQueryBuilder);
    assert!(sql_in.contains(r#""time" IN ($1, $2)"#));

    Ok(())
}
