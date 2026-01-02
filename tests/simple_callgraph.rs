use mr_hedgehog::infrastructure::SimpleCallGraphBuilder;
use mr_hedgehog::ports::CallGraphBuilder;

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

    let builder = SimpleCallGraphBuilder::new();
    let cg = builder.build_call_graph(&sources);
    let mut ids: Vec<String> = cg.nodes.iter().map(|n| n.id.clone()).collect();
    ids.sort();

    assert!(ids.contains(&"crate_one::foo".to_string()), "Expected foo, found: {:?}", ids);
    assert!(ids.contains(&"crate_one::bar".to_string()), "Expected bar, found: {:?}", ids);
    assert!(ids.contains(&"crate_two::baz".to_string()), "Expected baz, found: {:?}", ids);
}
