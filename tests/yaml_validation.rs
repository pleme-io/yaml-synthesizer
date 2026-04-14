/// Prove that yaml-synthesizer output is valid YAML via tree-sitter parser.
use yaml_synthesizer::*;

fn validate_yaml(source: &str) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_yaml::language().into())
        .expect("failed to set YAML language");
    let tree = parser.parse(source, None).expect("parser returned no tree");
    assert!(
        !tree.root_node().has_error(),
        "tree-sitter detected YAML parse error in:\n{source}"
    );
}

// ── Every YamlNode variant produces valid YAML ──────────────────────

#[test]
fn simple_map_valid_yaml() {
    let node = YamlNode::map(vec![
        ("name", YamlNode::str("test")),
        ("version", YamlNode::Int(1)),
        ("enabled", YamlNode::Bool(true)),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn nested_map_valid_yaml() {
    let node = YamlNode::map(vec![
        ("outer", YamlNode::map(vec![
            ("inner", YamlNode::map(vec![
                ("deep", YamlNode::Int(42)),
            ])),
        ])),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn sequence_valid_yaml() {
    let node = YamlNode::map(vec![
        ("items", YamlNode::seq(vec![
            YamlNode::str("a"),
            YamlNode::str("b"),
            YamlNode::str("c"),
        ])),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn seq_of_maps_valid_yaml() {
    let node = YamlNode::map(vec![
        ("steps", YamlNode::seq(vec![
            YamlNode::map(vec![("name", YamlNode::str("step1")), ("action", YamlNode::str("apply"))]),
            YamlNode::map(vec![("name", YamlNode::str("step2")), ("action", YamlNode::str("plan"))]),
        ])),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn mixed_types_valid_yaml() {
    let node = YamlNode::map(vec![
        ("string", YamlNode::str("hello")),
        ("integer", YamlNode::Int(42)),
        ("float", YamlNode::Float(3.14)),
        ("boolean", YamlNode::Bool(false)),
        ("null_val", YamlNode::Null),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn block_scalar_valid_yaml() {
    let node = YamlNode::map(vec![
        ("script", YamlNode::Block("echo hello\necho world".into())),
    ]);
    validate_yaml(&emit_file(&node));
}

#[test]
fn document_separator_valid_yaml() {
    let doc = emit_document(&YamlNode::map(vec![("key", YamlNode::str("value"))]));
    validate_yaml(&doc);
}

#[test]
fn fleet_builder_valid_yaml() {
    use yaml_synthesizer::builders::FleetBuilder;
    let fleet = FleetBuilder::new("deploy-quero")
        .description("Deploy quero platform")
        .step("dns", "quero-dns", "apply", vec![], vec![("DOMAIN", "quero.lol")])
        .step("builders", "quero-builders", "apply", vec!["dns"], vec![])
        .build();
    validate_yaml(&emit_file(&fleet));
}

#[test]
fn shikumi_config_valid_yaml() {
    use yaml_synthesizer::builders::ShikumiConfigBuilder;
    let config = ShikumiConfigBuilder::new()
        .string("domain", "quero.lol")
        .int("port", 8080)
        .bool("debug", false)
        .section("server", YamlNode::map(vec![
            ("host", YamlNode::str("0.0.0.0")),
            ("port", YamlNode::Int(3000)),
        ]))
        .build();
    validate_yaml(&emit_file(&config));
}

#[test]
fn deeply_nested_valid_yaml() {
    let node = YamlNode::map(vec![
        ("a", YamlNode::map(vec![
            ("b", YamlNode::map(vec![
                ("c", YamlNode::map(vec![
                    ("d", YamlNode::seq(vec![
                        YamlNode::map(vec![("e", YamlNode::Int(1))]),
                    ])),
                ])),
            ])),
        ])),
    ]);
    validate_yaml(&emit_file(&node));
}
