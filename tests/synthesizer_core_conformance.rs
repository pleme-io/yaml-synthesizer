//! Integration tests proving `YamlNode` conforms to `synthesizer_core` traits.
//!
//! Wave 2 of the compound-knowledge refactor. Every test calls one of
//! `synthesizer_core::node::laws::*` on a real `YamlNode` value, compounding
//! proof surface: the same laws prove properties of every synthesizer that
//! conforms.

use synthesizer_core::node::laws;
use synthesizer_core::{NoRawAttestation, SynthesizerNode};
use yaml_synthesizer::{YamlEntry, YamlNode};

// ─── Trait shape ────────────────────────────────────────────────────

#[test]
fn indent_unit_is_two_spaces() {
    assert_eq!(<YamlNode as SynthesizerNode>::indent_unit(), "  ");
}

#[test]
fn variant_ids_distinct_across_disjoint_variants() {
    let samples: Vec<YamlNode> = vec![
        YamlNode::Comment("c".into()),
        YamlNode::Blank,
        YamlNode::Str("s".into()),
        YamlNode::Int(42),
        YamlNode::Float(3.14),
        YamlNode::Bool(true),
        YamlNode::Null,
        YamlNode::Map(vec![]),
        YamlNode::Seq(vec![]),
        YamlNode::Block("line".into()),
        YamlNode::Folded("line".into()),
        YamlNode::TemplateExpr("{{ .Values.x }}".into()),
    ];
    let before = samples.len();
    let mut ids: Vec<u8> = samples.iter().map(SynthesizerNode::variant_id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        before,
        "variant_id must be distinct for disjoint variants"
    );
}

// ─── SynthesizerNode laws ───────────────────────────────────────────

#[test]
fn law_determinism_holds_on_simple_nodes() {
    for n in [
        YamlNode::Blank,
        YamlNode::Comment("x".into()),
        YamlNode::Str("hello".into()),
        YamlNode::Int(7),
        YamlNode::Bool(false),
        YamlNode::Null,
    ] {
        assert!(laws::is_deterministic(&n, 0));
        assert!(laws::is_deterministic(&n, 3));
    }
}

#[test]
fn law_determinism_holds_on_seq() {
    let n = YamlNode::Seq(vec![
        YamlNode::Str("a".into()),
        YamlNode::Str("b".into()),
    ]);
    assert!(laws::is_deterministic(&n, 2));
}

#[test]
fn law_determinism_holds_on_map() {
    let n = YamlNode::Map(vec![
        YamlEntry::new("name", YamlNode::Str("x".into())),
        YamlEntry::new("count", YamlNode::Int(42)),
    ]);
    assert!(laws::is_deterministic(&n, 1));
}

#[test]
fn law_honors_indent_unit_on_comment() {
    assert!(laws::honors_indent_unit(&YamlNode::Comment("hello".into()), 0));
    assert!(laws::honors_indent_unit(&YamlNode::Comment("hello".into()), 2));
}

#[test]
fn law_indent_monotone_len_on_str() {
    assert!(laws::indent_monotone_len(&YamlNode::Str("x".into()), 0));
    assert!(laws::indent_monotone_len(&YamlNode::Str("x".into()), 3));
}

#[test]
fn law_variant_id_valid_on_all_sample_variants() {
    let samples = [
        YamlNode::Comment("x".into()),
        YamlNode::Blank,
        YamlNode::Str("y".into()),
        YamlNode::Int(1),
        YamlNode::Float(2.0),
        YamlNode::Bool(true),
        YamlNode::Null,
        YamlNode::Map(vec![]),
        YamlNode::Seq(vec![]),
        YamlNode::TemplateExpr("{{ .Values.x }}".into()),
    ];
    for n in &samples {
        assert!(laws::variant_id_is_valid(n));
    }
}

// ─── NoRawAttestation ───────────────────────────────────────────────

#[test]
fn attestation_is_nonempty() {
    assert!(!<YamlNode as NoRawAttestation>::attestation().is_empty());
}

#[test]
fn attestation_mentions_raw() {
    let s = <YamlNode as NoRawAttestation>::attestation();
    assert!(
        s.to_lowercase().contains("raw"),
        "attestation must explain how no-raw is enforced — got: {s}"
    );
}

// ─── No-raw source invariant ────────────────────────────────────────

#[test]
fn no_raw_constructor_in_production_source() {
    // Scan src/ for `YamlNode::Raw(...)` or `Self::Raw(...)` constructor
    // uses. Legitimate non-constructions (variant declaration, match arms,
    // #[allow(deprecated)]-pinned references, comments, attribute lines)
    // are exempted.
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in walk_rust_files(&src_dir) {
        let content = std::fs::read_to_string(&path).expect("read src file");
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            // Variant declaration line.
            if line.contains("Raw(String)") {
                continue;
            }
            // Match arms (patterns, not constructions).
            if line.contains("=>") {
                continue;
            }
            // Attribute lines.
            if trimmed.starts_with("#[") {
                continue;
            }
            // Preceding #[allow(deprecated)] → intentional reference.
            let prev_allows = i > 0 && lines[i - 1].contains("#[allow(deprecated)]");
            if prev_allows {
                continue;
            }
            if line.contains("YamlNode::Raw(") || line.contains("Self::Raw(") {
                violations.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "YamlNode::Raw construction in production source is forbidden \
         (use a typed variant). Violations: {violations:?}"
    );
}

fn walk_rust_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).expect("read src dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rust_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}
