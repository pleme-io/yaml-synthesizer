use proptest::prelude::*;
use yaml_synthesizer::*;

fn arb_scalar() -> impl Strategy<Value = YamlNode> {
    prop_oneof![
        any::<i64>().prop_map(YamlNode::Int),
        any::<bool>().prop_map(YamlNode::Bool),
        Just(YamlNode::Null),
        "[a-z][a-z0-9_]{0,10}".prop_map(|s| YamlNode::Str(s)),
    ]
}

proptest! {
    #[test]
    fn scalar_emit_does_not_panic(node in arb_scalar()) {
        let _ = node.emit(0);
    }

    #[test]
    fn scalar_emit_at_any_indent(node in arb_scalar(), indent in 0usize..10) {
        let _ = node.emit(indent);
    }

    #[test]
    fn emit_deterministic(node in arb_scalar()) {
        let a = node.emit(0);
        let b = node.emit(0);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn emit_file_trailing_newline(node in arb_scalar()) {
        let out = emit_file(&node);
        prop_assert!(out.ends_with('\n'));
    }

    #[test]
    fn emit_document_starts_with_separator(node in arb_scalar()) {
        let out = emit_document(&node);
        prop_assert!(out.starts_with("---\n"));
    }

    #[test]
    fn no_control_chars_except_newline(node in arb_scalar()) {
        let out = node.emit(0);
        for ch in out.chars() {
            if ch.is_control() {
                prop_assert!(ch == '\n' || ch == '\t', "unexpected control char");
            }
        }
    }

    #[test]
    fn indentation_multiples_of_two(
        keys in proptest::collection::vec("[a-z]{1,3}", 1..3),
        values in proptest::collection::vec(any::<i64>(), 1..3)
    ) {
        let len = keys.len().min(values.len());
        let entries: Vec<YamlEntry> = keys[..len].iter().zip(&values[..len])
            .map(|(k, v)| YamlEntry::new(k, YamlNode::Int(*v)))
            .collect();
        let node = YamlNode::Map(entries);
        let out = node.emit(0);
        for line in out.lines() {
            if line.starts_with(' ') {
                let spaces = line.len() - line.trim_start().len();
                prop_assert!(spaces % 2 == 0, "indentation must be multiples of 2");
            }
        }
    }

    #[test]
    fn string_quoting_preserves_content(s in "[a-zA-Z]{1,10}") {
        let out = YamlNode::str(&s).emit(0);
        // The original string content must be recoverable (present in output)
        prop_assert!(out.contains(&s), "content must be preserved");
    }
}
