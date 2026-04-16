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
            #[allow(deprecated)]
            Self::Raw(_) => 12,
        }
    }
}

impl NoRawAttestation for YamlNode {
    fn attestation() -> &'static str {
        "YamlNode::Raw carries #[deprecated] in src/node.rs and is scheduled \
         for removal in Wave 3 of the compound-knowledge refactor. \
         tests/no_raw_invariant.rs::no_raw_in_production_code and \
         tests/synthesizer_core_conformance.rs::no_raw_constructor_in_production_source \
         scan src/ for Raw constructions; any accidental reintroduction \
         fails CI. TemplateExpr is a typed Helm/Go template bridge, not a \
         raw escape hatch. The #[allow(deprecated)] pin in \
         synthesizer_core_impl.rs and node.rs emit() is the one intentional \
         reference — a match arm pattern, not a construction."
    }
}
