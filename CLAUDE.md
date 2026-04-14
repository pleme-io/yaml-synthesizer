# yaml-synthesizer

Typed AST for structurally correct YAML generation. Foundation for helm-synthesizer and kustomize-synthesizer.

## Tests: 78 | Status: Proven

## Core API

| Type | Purpose |
|------|---------|
| `YamlNode` | 12 variants: Comment, Blank, Str, Int, Float, Bool, Null, Map, Seq, Block, Folded, Raw |
| `YamlEntry` | Key-value pair with optional inline comment |
| `emit_file(&YamlNode)` | Emit without `---` separator |
| `emit_document(&YamlNode)` | Emit with `---` separator |
| `emit_multi_document(&[YamlNode])` | Multiple docs separated by `---` |

## Builders

- `FleetBuilder` — fleet.yaml for Pangea deployment flows with DAG-ordered steps
- `ShikumiConfigBuilder` — shikumi config YAML with typed sections

## String Quoting

Auto-quotes values that look like YAML keywords (true/false/yes/no/null/~), numbers, or contain special characters. Preserves content through quoting.

## Dependencies

None (zero runtime deps). proptest (dev).
