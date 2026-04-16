use yaml_synthesizer::*;

// ── Every YamlNode variant emits correctly ──────────────────────────

#[test]
fn comment_emits_hash() {
    assert_eq!(YamlNode::Comment("test".into()).emit(0), "# test");
}

#[test]
fn blank_emits_empty() {
    assert_eq!(YamlNode::Blank.emit(0), "");
}

#[test]
fn str_simple_no_quotes() {
    assert_eq!(YamlNode::str("hello").emit(0), "hello");
}

#[test]
fn str_needs_quoting_for_true() {
    assert_eq!(YamlNode::str("true").emit(0), "\"true\"");
}

#[test]
fn str_needs_quoting_for_false() {
    assert_eq!(YamlNode::str("false").emit(0), "\"false\"");
}

#[test]
fn str_needs_quoting_for_yes() {
    assert_eq!(YamlNode::str("yes").emit(0), "\"yes\"");
}

#[test]
fn str_needs_quoting_for_no() {
    assert_eq!(YamlNode::str("no").emit(0), "\"no\"");
}

#[test]
fn str_needs_quoting_for_null() {
    assert_eq!(YamlNode::str("null").emit(0), "\"null\"");
}

#[test]
fn str_needs_quoting_for_numbers() {
    assert_eq!(YamlNode::str("123").emit(0), "\"123\"");
    assert_eq!(YamlNode::str("3.14").emit(0), "\"3.14\"");
}

#[test]
fn str_needs_quoting_for_empty() {
    assert_eq!(YamlNode::str("").emit(0), "\"\"");
}

#[test]
fn str_escapes_inner_quotes() {
    let out = YamlNode::str("say \"hi\"").emit(0);
    assert!(out.contains("\\\""));
}

#[test]
fn int_emits_number() {
    assert_eq!(YamlNode::Int(42).emit(0), "42");
    assert_eq!(YamlNode::Int(-1).emit(0), "-1");
    assert_eq!(YamlNode::Int(0).emit(0), "0");
}

#[test]
fn float_emits_decimal() {
    assert_eq!(YamlNode::Float(3.14).emit(0), "3.14");
}

#[test]
fn float_whole_number_has_decimal() {
    assert_eq!(YamlNode::Float(1.0).emit(0), "1.0");
}

#[test]
fn bool_emits_lowercase() {
    assert_eq!(YamlNode::Bool(true).emit(0), "true");
    assert_eq!(YamlNode::Bool(false).emit(0), "false");
}

#[test]
fn null_emits_null() {
    assert_eq!(YamlNode::Null.emit(0), "null");
}

#[test]
fn empty_map_emits_braces() {
    assert_eq!(YamlNode::Map(vec![]).emit(0), "{}");
}

#[test]
fn simple_map() {
    let node = YamlNode::map(vec![("key", YamlNode::str("value"))]);
    assert_eq!(node.emit(0), "key: value");
}

#[test]
fn map_multiple_entries() {
    let node = YamlNode::map(vec![
        ("a", YamlNode::Int(1)),
        ("b", YamlNode::str("two")),
        ("c", YamlNode::Bool(true)),
    ]);
    let out = node.emit(0);
    assert!(out.contains("a: 1"));
    assert!(out.contains("b: two"));
    assert!(out.contains("c: true"));
}

#[test]
fn nested_map() {
    let node = YamlNode::map(vec![(
        "outer",
        YamlNode::map(vec![("inner", YamlNode::Int(1))]),
    )]);
    let out = node.emit(0);
    assert!(out.contains("outer:"));
    assert!(out.contains("  inner: 1"));
}

#[test]
fn empty_seq_emits_brackets() {
    assert_eq!(YamlNode::Seq(vec![]).emit(0), "[]");
}

#[test]
fn simple_seq() {
    let node = YamlNode::seq(vec![YamlNode::Int(1), YamlNode::Int(2), YamlNode::Int(3)]);
    let out = node.emit(0);
    assert!(out.contains("- 1"));
    assert!(out.contains("- 2"));
    assert!(out.contains("- 3"));
}

#[test]
fn seq_of_maps() {
    let node = YamlNode::seq(vec![
        YamlNode::map(vec![("name", YamlNode::str("a"))]),
        YamlNode::map(vec![("name", YamlNode::str("b"))]),
    ]);
    let out = node.emit(0);
    assert!(out.contains("- name: a"));
    assert!(out.contains("- name: b"));
}

#[test]
fn block_scalar() {
    let node = YamlNode::map(vec![
        ("script", YamlNode::Block("echo hello\necho world".into())),
    ]);
    let out = node.emit(0);
    assert!(out.contains("script: |"));
    assert!(out.contains("  echo hello"));
    assert!(out.contains("  echo world"));
}

#[test]
fn folded_scalar() {
    let node = YamlNode::map(vec![
        ("desc", YamlNode::Folded("line one\nline two".into())),
    ]);
    let out = node.emit(0);
    assert!(out.contains("desc: >"));
}

#[test]
fn map_with_inline_comment() {
    let entry = YamlEntry::new("port", YamlNode::Int(8080)).with_comment("TCP port");
    let node = YamlNode::Map(vec![entry]);
    let out = node.emit(0);
    assert!(out.contains("port: 8080  # TCP port"));
}

// ── Indentation proofs ──────────────────────────────────────────────

#[test]
fn indent_level_0() {
    assert_eq!(YamlNode::str("test").emit(0), "test");
}

#[test]
fn indent_level_1() {
    assert_eq!(YamlNode::str("test").emit(1), "  test");
}

#[test]
fn indent_level_2() {
    assert_eq!(YamlNode::str("test").emit(2), "    test");
}

#[test]
fn nested_indentation_increases() {
    let node = YamlNode::map(vec![(
        "a",
        YamlNode::map(vec![("b", YamlNode::map(vec![("c", YamlNode::Int(1))]))]),
    )]);
    let out = node.emit(0);
    let lines: Vec<&str> = out.lines().collect();
    // c should be more indented than b, which is more indented than a
    let a_indent = lines.iter().find(|l| l.contains("a:")).map(|l| l.len() - l.trim_start().len()).unwrap();
    let b_indent = lines.iter().find(|l| l.contains("b:")).map(|l| l.len() - l.trim_start().len()).unwrap();
    let c_indent = lines.iter().find(|l| l.contains("c:")).map(|l| l.len() - l.trim_start().len()).unwrap();
    assert!(b_indent > a_indent);
    assert!(c_indent > b_indent);
}

// ── Emitter proofs ──────────────────────────────────────────────────

#[test]
fn document_starts_with_separator() {
    let out = emit_document(&YamlNode::map(vec![("x", YamlNode::Int(1))]));
    assert!(out.starts_with("---\n"));
}

#[test]
fn document_trailing_newline() {
    let out = emit_document(&YamlNode::Null);
    assert!(out.ends_with('\n'));
}

#[test]
fn file_no_separator() {
    let out = emit_file(&YamlNode::Int(1));
    assert!(!out.starts_with("---"));
}

#[test]
fn file_trailing_newline() {
    let out = emit_file(&YamlNode::Int(1));
    assert!(out.ends_with('\n'));
}

#[test]
fn multi_document_count() {
    let docs = vec![YamlNode::Int(1), YamlNode::Int(2), YamlNode::Int(3)];
    let out = emit_multi_document(&docs);
    assert_eq!(out.matches("---").count(), 3);
}

#[test]
fn deterministic_emit() {
    let node = YamlNode::map(vec![
        ("a", YamlNode::Int(1)),
        ("b", YamlNode::seq(vec![YamlNode::str("x"), YamlNode::str("y")])),
        ("c", YamlNode::map(vec![("d", YamlNode::Bool(true))])),
    ]);
    let a = emit_file(&node);
    let b = emit_file(&node);
    assert_eq!(a, b);
}

// ── Builder proofs ──────────────────────────────────────────────────

#[test]
fn fleet_builder_has_name() {
    use yaml_synthesizer::builders::FleetBuilder;
    let fleet = FleetBuilder::new("deploy-quero").build();
    let out = emit_file(&fleet);
    assert!(out.contains("name: deploy-quero"));
}

#[test]
fn fleet_builder_has_steps() {
    use yaml_synthesizer::builders::FleetBuilder;
    let fleet = FleetBuilder::new("test")
        .step("dns", "quero-dns", "apply", vec![], vec![])
        .build();
    let out = emit_file(&fleet);
    assert!(out.contains("steps:"));
    assert!(out.contains("name: dns"));
    assert!(out.contains("workspace: quero-dns"));
    assert!(out.contains("action: apply"));
}

#[test]
fn fleet_builder_dependencies() {
    use yaml_synthesizer::builders::FleetBuilder;
    let fleet = FleetBuilder::new("test")
        .step("a", "ws-a", "apply", vec![], vec![])
        .step("b", "ws-b", "apply", vec!["a"], vec![])
        .build();
    let out = emit_file(&fleet);
    assert!(out.contains("depends_on:"));
}

#[test]
fn fleet_builder_env() {
    use yaml_synthesizer::builders::FleetBuilder;
    let fleet = FleetBuilder::new("test")
        .step("dns", "quero-dns", "apply", vec![], vec![("DOMAIN", "quero.lol")])
        .build();
    let out = emit_file(&fleet);
    assert!(out.contains("DOMAIN: quero.lol"));
}

#[test]
fn shikumi_builder_simple() {
    use yaml_synthesizer::builders::ShikumiConfigBuilder;
    let config = ShikumiConfigBuilder::new()
        .string("domain", "quero.lol")
        .int("port", 8080)
        .bool("debug", false)
        .build();
    let out = emit_file(&config);
    assert!(out.contains("domain: quero.lol"));
    assert!(out.contains("port: 8080"));
    assert!(out.contains("debug: false"));
}
