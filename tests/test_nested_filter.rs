use modql::filter::{FilterNodes, IntoFilterNodes, OpValsInt64, OpValsString};

#[derive(Clone, FilterNodes, Default)]
pub struct SubFilter {
	name: Option<OpValsString>,
}

#[derive(Clone, FilterNodes, Default)]
pub struct MainFilter {
	id: Option<OpValsInt64>,
	#[modql(nested)]
	sub: Option<SubFilter>,
}

#[test]
fn test_nested_filter_nodes() {
	let filter = MainFilter {
		id: Some(123.into()),
		sub: Some(SubFilter {
			name: Some("test".into()),
		}),
	};

	let nodes: Vec<modql::filter::FilterNode> = filter.into();
	assert_eq!(nodes.len(), 2);
	assert_eq!(nodes[0].name, "id");
	assert_eq!(nodes[1].name, "name");
}
