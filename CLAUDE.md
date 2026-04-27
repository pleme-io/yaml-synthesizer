# yaml-synthesizer

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


Typed AST for structurally correct YAML generation. Foundation for helm-synthesizer and kustomize-synthesizer. All output validated by tree-sitter-yaml parser.

## Tests: 99 | Status: Proven, tree-sitter Validated, No Raw (Structural)

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
