use crate::node::YamlNode;

/// Emit a YAML document from a top-level node.
///
/// For documents starting with a mapping, emits `---` separator.
/// Deterministic: identical ASTs produce byte-identical output.
/// Always ends with exactly one trailing newline.
#[must_use]
pub fn emit_document(root: &YamlNode) -> String {
    let body = root.emit(0);
    let mut out = format!("---\n{body}");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Emit a YAML document without the `---` separator.
#[must_use]
pub fn emit_file(root: &YamlNode) -> String {
    let mut out = root.emit(0);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Emit multiple YAML documents separated by `---`.
#[must_use]
pub fn emit_multi_document(docs: &[YamlNode]) -> String {
    let mut parts = Vec::new();
    for doc in docs {
        parts.push(format!("---\n{}", doc.emit(0)));
    }
    let mut out = parts.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::YamlNode;

    #[test]
    fn document_starts_with_separator() {
        let doc = emit_document(&YamlNode::map(vec![("key", YamlNode::str("value"))]));
        assert!(doc.starts_with("---\n"));
    }

    #[test]
    fn document_trailing_newline() {
        let doc = emit_document(&YamlNode::Int(42));
        assert!(doc.ends_with('\n'));
    }

    #[test]
    fn file_no_separator() {
        let out = emit_file(&YamlNode::map(vec![("x", YamlNode::Int(1))]));
        assert!(!out.starts_with("---"));
    }

    #[test]
    fn file_trailing_newline() {
        let out = emit_file(&YamlNode::Null);
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn multi_document_separators() {
        let docs = vec![YamlNode::Int(1), YamlNode::Int(2)];
        let out = emit_multi_document(&docs);
        assert_eq!(out.matches("---").count(), 2);
    }

    #[test]
    fn deterministic() {
        let node = YamlNode::map(vec![("a", YamlNode::Int(1)), ("b", YamlNode::str("hello"))]);
        let a = emit_file(&node);
        let b = emit_file(&node);
        assert_eq!(a, b);
    }
}
