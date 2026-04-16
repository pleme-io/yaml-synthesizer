//! Conformance to [`synthesizer_core`] traits.
//!
//! Wave 2 of the compound-knowledge refactor: purely additive. No behavior
//! change to yaml-synthesizer's existing APIs — this module only adds trait
//! impls that downstream generic code can consume.

use crate::node::YamlNode;
use synthesizer_core::{NoRawAttestation, SynthesizerNode};

impl SynthesizerNode for YamlNode {
    fn emit(&self, indent: usize) -> String {
        // Delegate to the inherent `YamlNode::emit` — inherent takes
        // priority over trait methods in UFCS path lookup.
        YamlNode::emit(self, indent)
    }

    fn indent_unit() -> &'static str {
        "  "
    }

    fn variant_id(&self) -> u8 {
        match self {
            Self::Comment(_) => 0,
            Self::Blank => 1,
            Self::Str(_) => 2,
            Self::Int(_) => 3,
            Self::Float(_) => 4,
            Self::Bool(_) => 5,
            Self::Null => 6,
            Self::Map(_) => 7,
            Self::Seq(_) => 8,
            Self::Block(_) => 9,
            Self::Folded(_) => 10,
            Self::TemplateExpr(_) => 11,
        }
    }
}

impl NoRawAttestation for YamlNode {
    fn attestation() -> &'static str {
        "YamlNode::Raw was REMOVED in Wave 3 of the compound-knowledge \
         refactor — the no-raw invariant is now STRUCTURAL, not \
         documentary. YamlNode can no longer represent arbitrary strings; \
         invalid states are unrepresentable at the type level. The \
         defensive scanners in tests/no_raw_invariant.rs::no_raw_in_production_code \
         and tests/synthesizer_core_conformance.rs::no_raw_constructor_in_production_source \
         remain as a second line of defense against any accidental \
         reintroduction. TemplateExpr is the typed Helm/Go template \
         bridge and is NOT a raw escape hatch — it declares intent at \
         the type level."
    }
}
