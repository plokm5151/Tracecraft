use tracecraft::infrastructure::SimpleCallGraphBuilder;
use tracecraft::ports::CallGraphBuilder;

#[test]
fn node_ids_include_crate_names() {
    // Build an in-memory set of source files across two crates
    let crate_one = r#"
        fn foo() {}
        fn bar() { foo(); }
    "#;
    let crate_two = r#"
        fn baz() {}
    "#;

    let sources = vec![
        ("crate_one".to_string(), "lib.rs".to_string(), crate_one.to_string()),
        ("crate_two".to_string(), "lib.rs".to_string(), crate_two.to_string()),
    ];

    let builder = SimpleCallGraphBuilder;
    let cg = builder.build_call_graph(&sources);
    let mut ids: Vec<String> = cg.nodes.iter().map(|n| n.id.clone()).collect();
    ids.sort();

    assert!(ids.contains(&"foo@crate_one".to_string()));
    assert!(ids.contains(&"bar@crate_one".to_string()));
    assert!(ids.contains(&"baz@crate_two".to_string()));
}
