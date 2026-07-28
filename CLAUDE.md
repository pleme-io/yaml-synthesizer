# yaml-synthesizer

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


Typed AST for structurally correct YAML generation. Foundation for helm-synthesizer and kustomize-synthesizer. All output validated by tree-sitter-yaml parser.

## Tests: 109 | Status: Proven, tree-sitter Validated, No Raw (Structural)

Reproduce — this is the CI gate in `.github/workflows/ci.yml`:

```sh
cargo test --all-targets --all-features   # 109 passed, 0 failed
cargo test --doc         --all-features   #   0 (no doc examples yet)
```

`--all-targets` is required: without it a non-compiling test target can hide
behind a green `cargo build`. It also excludes doctests, hence the second
command.

| Tests | Where | What |
|------:|-------|------|
| 41 | `tests/exhaustive_proofs.rs` | Every `YamlNode` variant emits correctly |
| 16 | `src/node.rs` | Per-variant node emission |
| 11 | `tests/synthesizer_core_conformance.rs` | `YamlNode` conformance to `synthesizer_core` traits |
| 10 | `tests/yaml_validation.rs` | tree-sitter-yaml parse of every output pattern |
| 10 | `src/flow_dag.rs` | DAG ordering for `FleetBuilder` steps |
| 8 | `tests/properties.rs` | proptest invariants over arbitrary trees |
| 6 | `src/emitter.rs` | Document / multi-document emission |
| 6 | `src/builders.rs` | `FleetBuilder` + `ShikumiConfigBuilder` output |
| 1 | `tests/no_raw_invariant.rs` | INVARIANT: zero `YamlNode::Raw` constructors in production source |

## Core API

| Type | Purpose |
|------|---------|
| `YamlNode` | 12 variants: Comment, Blank, Str, Int, Float, Bool, Null, Map, Seq, Block, Folded, TemplateExpr |
| `YamlEntry` | Key-value pair with optional inline comment |
| `emit_file(&YamlNode)` | Emit without `---` separator |
| `emit_document(&YamlNode)` | Emit with `---` separator |
| `emit_multi_document(&[YamlNode])` | Multiple docs separated by `---` |

`TemplateExpr` — typed bridge for Helm Go template expressions. NOT an escape hatch.

## Builders

- `FleetBuilder` — fleet.yaml for Pangea deployment flows with DAG-ordered steps
- `ShikumiConfigBuilder` — shikumi config YAML with typed sections

## tree-sitter Validation

10 tests validate every output pattern (maps, sequences, nested, blocks) via `tree-sitter-yaml`.

## No-Raw Invariant

Test scans production source for YamlNode::Raw constructors → assert zero.
