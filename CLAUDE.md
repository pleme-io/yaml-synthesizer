# yaml-synthesizer

Typed AST for structurally correct YAML generation. Foundation for helm-synthesizer and kustomize-synthesizer. All output validated by tree-sitter-yaml parser.

## Tests: 89 | Status: Proven, tree-sitter Validated, Zero Raw in Production

## Core API

| Type | Purpose |
|------|---------|
| `YamlNode` | 13 variants: Comment, Blank, Str, Int, Float, Bool, Null, Map, Seq, Block, Folded, TemplateExpr, ~~Raw~~ (deprecated) |
| `YamlEntry` | Key-value pair with optional inline comment |
| `emit_file(&YamlNode)` | Emit without `---` separator |
| `emit_document(&YamlNode)` | Emit with `---` separator |
| `emit_multi_document(&[YamlNode])` | Multiple docs separated by `---` |

`TemplateExpr` — typed bridge for Helm Go template expressions. NOT an escape hatch.
`Raw` — **deprecated**. Use TemplateExpr or a typed variant.

## Builders

- `FleetBuilder` — fleet.yaml for Pangea deployment flows with DAG-ordered steps
- `ShikumiConfigBuilder` — shikumi config YAML with typed sections

## tree-sitter Validation

10 tests validate every output pattern (maps, sequences, nested, blocks) via `tree-sitter-yaml`.

## No-Raw Invariant

Test scans production source for YamlNode::Raw constructors → assert zero.
