/// Every YAML construct that yaml-synthesizer can emit.
///
/// Pure data, no IO. `emit()` is deterministic.
#[derive(Debug, Clone, PartialEq)]
pub enum YamlNode {
    /// Comment: `# text`
    Comment(String),
    /// Blank line
    Blank,
    /// Scalar string (auto-quotes if needed)
    Str(String),
    /// Integer
    Int(i64),
    /// Float
    Float(f64),
    /// Boolean
    Bool(bool),
    /// Null
    Null,
    /// Mapping (ordered key-value pairs)
    Map(Vec<YamlEntry>),
    /// Sequence (list)
    Seq(Vec<YamlNode>),
    /// Multi-line string (block scalar `|`)
    Block(String),
    /// Folded string (block scalar `>`)
    Folded(String),
    /// Raw YAML (escape hatch)
    Raw(String),
}

/// A key-value pair in a YAML mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct YamlEntry {
    pub key: String,
    pub value: YamlNode,
    pub comment: Option<String>,
}

impl YamlEntry {
    #[must_use]
    pub fn new(key: &str, value: YamlNode) -> Self {
        Self {
            key: key.to_string(),
            value,
            comment: None,
        }
    }

    #[must_use]
    pub fn with_comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}

impl YamlNode {
    #[must_use]
    pub fn str(s: &str) -> Self {
        Self::Str(s.to_string())
    }

    #[must_use]
    pub fn map(entries: Vec<(&str, YamlNode)>) -> Self {
        Self::Map(
            entries
                .into_iter()
                .map(|(k, v)| YamlEntry::new(k, v))
                .collect(),
        )
    }

    #[must_use]
    pub fn seq(items: Vec<YamlNode>) -> Self {
        Self::Seq(items)
    }

    /// Emit this node as YAML at the given indentation level (2 spaces per level).
    #[must_use]
    pub fn emit(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        match self {
            Self::Comment(text) => format!("{pad}# {text}"),
            Self::Blank => String::new(),
            Self::Str(s) => {
                if needs_quoting(s) {
                    let escaped = s.replace('"', "\\\"");
                    format!("{pad}\"{escaped}\"")
                } else {
                    format!("{pad}{s}")
                }
            }
            Self::Int(n) => format!("{pad}{n}"),
            Self::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{pad}{f:.1}")
                } else {
                    format!("{pad}{f}")
                }
            }
            Self::Bool(b) => format!("{pad}{b}"),
            Self::Null => format!("{pad}null"),
            Self::Map(entries) => {
                if entries.is_empty() {
                    return format!("{pad}{{}}");
                }
                let mut lines = Vec::new();
                for entry in entries {
                    let comment_suffix = entry
                        .comment
                        .as_ref()
                        .map(|c| format!("  # {c}"))
                        .unwrap_or_default();

                    match &entry.value {
                        // Inline scalars
                        Self::Str(_) | Self::Int(_) | Self::Float(_) | Self::Bool(_)
                        | Self::Null | Self::Raw(_) => {
                            let val = entry.value.emit(0);
                            lines.push(format!("{pad}{}: {val}{comment_suffix}", entry.key));
                        }
                        // Block scalar
                        Self::Block(text) => {
                            lines.push(format!("{pad}{}: |{comment_suffix}", entry.key));
                            for line in text.lines() {
                                if line.is_empty() {
                                    lines.push(String::new());
                                } else {
                                    lines.push(format!("{pad}  {line}"));
                                }
                            }
                        }
                        Self::Folded(text) => {
                            lines.push(format!("{pad}{}: >{comment_suffix}", entry.key));
                            for line in text.lines() {
                                if line.is_empty() {
                                    lines.push(String::new());
                                } else {
                                    lines.push(format!("{pad}  {line}"));
                                }
                            }
                        }
                        // Nested map/seq
                        Self::Map(_) | Self::Seq(_) => {
                            lines.push(format!("{pad}{}:{comment_suffix}", entry.key));
                            lines.push(entry.value.emit(indent + 1));
                        }
                        // Comments/blanks shouldn't be values but handle gracefully
                        Self::Comment(_) | Self::Blank => {
                            lines.push(format!("{pad}{}:{comment_suffix}", entry.key));
                        }
                    }
                }
                lines.join("\n")
            }
            Self::Seq(items) => {
                if items.is_empty() {
                    return format!("{pad}[]");
                }
                let mut lines = Vec::new();
                for item in items {
                    match item {
                        Self::Map(_) | Self::Seq(_) => {
                            // First key of nested map goes on same line as `-`
                            let inner = item.emit(indent + 1);
                            let trimmed = inner.trim_start();
                            lines.push(format!("{pad}- {trimmed}"));
                        }
                        _ => {
                            let val = item.emit(0);
                            lines.push(format!("{pad}- {val}"));
                        }
                    }
                }
                lines.join("\n")
            }
            Self::Block(text) => {
                let mut lines = vec![format!("{pad}|")];
                for line in text.lines() {
                    if line.is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.push(format!("{pad}  {line}"));
                    }
                }
                lines.join("\n")
            }
            Self::Folded(text) => {
                let mut lines = vec![format!("{pad}>")];
                for line in text.lines() {
                    if line.is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.push(format!("{pad}  {line}"));
                    }
                }
                lines.join("\n")
            }
            Self::Raw(s) => format!("{pad}{s}"),
        }
    }
}

/// Check if a string needs quoting in YAML.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    // YAML special values
    let lower = s.to_lowercase();
    if matches!(
        lower.as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~"
    ) {
        return true;
    }
    // Starts with special characters
    if s.starts_with(|c: char| matches!(c, '{' | '[' | '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '!' | '%' | '@' | '`' | '#' | ','))
    {
        return true;
    }
    // Contains : followed by space, or has newlines
    if s.contains(": ") || s.contains('\n') || s.contains('"') {
        return true;
    }
    // Looks numeric
    if s.parse::<f64>().is_ok() || s.parse::<i64>().is_ok() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_emits_hash() {
        assert_eq!(YamlNode::Comment("test".into()).emit(0), "# test");
    }

    #[test]
    fn blank_emits_empty() {
        assert_eq!(YamlNode::Blank.emit(0), "");
    }

    #[test]
    fn str_simple() {
        assert_eq!(YamlNode::str("hello").emit(0), "hello");
    }

    #[test]
    fn str_quotes_when_needed() {
        assert_eq!(YamlNode::str("true").emit(0), "\"true\"");
        assert_eq!(YamlNode::str("").emit(0), "\"\"");
        assert_eq!(YamlNode::str("123").emit(0), "\"123\"");
    }

    #[test]
    fn int_emits_number() {
        assert_eq!(YamlNode::Int(42).emit(0), "42");
    }

    #[test]
    fn float_emits_number() {
        assert_eq!(YamlNode::Float(3.14).emit(0), "3.14");
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
    fn empty_map() {
        assert_eq!(YamlNode::Map(vec![]).emit(0), "{}");
    }

    #[test]
    fn simple_map() {
        let node = YamlNode::map(vec![("key", YamlNode::str("value"))]);
        assert_eq!(node.emit(0), "key: value");
    }

    #[test]
    fn empty_seq() {
        assert_eq!(YamlNode::Seq(vec![]).emit(0), "[]");
    }

    #[test]
    fn simple_seq() {
        let node = YamlNode::seq(vec![YamlNode::Int(1), YamlNode::Int(2)]);
        let out = node.emit(0);
        assert!(out.contains("- 1"));
        assert!(out.contains("- 2"));
    }

    #[test]
    fn nested_map() {
        let node = YamlNode::map(vec![
            ("outer", YamlNode::map(vec![("inner", YamlNode::Int(1))])),
        ]);
        let out = node.emit(0);
        assert!(out.contains("outer:"));
        assert!(out.contains("inner: 1"));
    }

    #[test]
    fn map_with_comment() {
        let entry = YamlEntry::new("port", YamlNode::Int(8080))
            .with_comment("TCP port");
        let node = YamlNode::Map(vec![entry]);
        let out = node.emit(0);
        assert!(out.contains("# TCP port"));
    }

    #[test]
    fn block_scalar() {
        let node = YamlNode::map(vec![
            ("script", YamlNode::Block("echo hello\necho world".into())),
        ]);
        let out = node.emit(0);
        assert!(out.contains("script: |"));
        assert!(out.contains("echo hello"));
    }

    #[test]
    fn indent_propagates() {
        let out = YamlNode::Comment("test".into()).emit(2);
        assert_eq!(out, "    # test");
    }
}
