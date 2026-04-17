use crate::node::{YamlEntry, YamlNode};

// ── FleetBuilder ──────────────────────────────────────────────────���─

/// Build a structurally correct fleet.yaml for Pangea deployment flows.
///
/// Single-flow mode is the default for backwards compat — `.step()` and
/// `.build()` emit the `name: / steps: [...]` top-level shape. The
/// multi-flow mode via `.add_flow(FleetFlow)` emits a `flows:` map with
/// multiple named flows, matching the fleet-tool schema.
pub struct FleetBuilder {
    name: String,
    description: Option<String>,
    steps: Vec<FleetStep>,
    flows: Vec<FleetFlow>,
}

/// One named flow — a DAG of steps with dependencies + actions.
/// Actions are typed via [`FleetAction`]; callers pick a variant per step.
pub struct FleetFlow {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<FleetFlowStep>,
}

/// A step inside a [`FleetFlow`]. Action shape = one of the fleet-tool
/// action types (shell, pangea-op, aws-cli, packer-build, etc.).
pub struct FleetFlowStep {
    pub id: String,
    pub action: FleetAction,
    pub depends_on: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Typed action — discriminated union over what a fleet step can do.
/// Serializes as `action: { type: <variant>, ...fields }`.
#[derive(Clone)]
pub enum FleetAction {
    /// Opaque shell command. Use when a step is a pre-existing nix app
    /// call or a tiny glue snippet that doesn't fit other variants.
    Shell { command: String },
    /// Pangea workspace op — maps to `nix run .#<op>` where `<op>` is
    /// plan / apply / destroy / synth.
    PangeaOp { op: String },
    /// Structured AWS CLI invocation. Rendered as a shell action that
    /// exec's `aws <service> <subcmd>` with each flag = shell arg.
    AwsCli { service: String, subcommand: String, args: Vec<(String, String)> },
    /// Packer build against a typed packer.json path with typed `-var`
    /// flags. `vars` values are LITERAL (pre-resolved) — if you need
    /// dynamic values, use a prior Shell/AwsCli step to discover them
    /// and a Shell step here.
    PackerBuild { packer_json: String, vars: Vec<(String, String)> },
    /// Reference another flow by name — DAG-of-DAGs / flow-of-flows
    /// composition. The referenced flow's steps expand into the parent
    /// flow's DAG at run time, with this step's id as the expansion
    /// scope. `params` substitute into the referenced flow's
    /// parameterized placeholders.
    SubFlow { flow: String, params: Vec<(String, String)> },
}

struct FleetStep {
    name: String,
    workspace: String,
    action: String,
    depends_on: Vec<String>,
    env: Vec<(String, String)>,
}

impl FleetBuilder {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            steps: Vec::new(),
            flows: Vec::new(),
        }
    }

    /// Add a named flow (multi-flow mode). When any flows are added,
    /// `.build()` emits the fleet-tool `flows:` schema instead of the
    /// single-flow `name:/steps:` shape.
    #[must_use]
    pub fn add_flow(mut self, flow: FleetFlow) -> Self {
        self.flows.push(flow);
        self
    }

    #[must_use]
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    #[must_use]
    pub fn step(
        mut self,
        name: &str,
        workspace: &str,
        action: &str,
        depends_on: Vec<&str>,
        env: Vec<(&str, &str)>,
    ) -> Self {
        self.steps.push(FleetStep {
            name: name.to_string(),
            workspace: workspace.to_string(),
            action: action.to_string(),
            depends_on: depends_on.into_iter().map(|s| s.to_string()).collect(),
            env: env
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
        self
    }

    #[must_use]
    pub fn build(&self) -> YamlNode {
        let mut entries = vec![YamlEntry::new("name", YamlNode::str(&self.name))];

        if let Some(ref desc) = self.description {
            entries.push(YamlEntry::new("description", YamlNode::str(desc)));
        }

        let steps: Vec<YamlNode> = self
            .steps
            .iter()
            .map(|s| {
                let mut step_entries = vec![
                    YamlEntry::new("name", YamlNode::str(&s.name)),
                    YamlEntry::new("workspace", YamlNode::str(&s.workspace)),
                    YamlEntry::new("action", YamlNode::str(&s.action)),
                ];

                if !s.depends_on.is_empty() {
                    step_entries.push(YamlEntry::new(
                        "depends_on",
                        YamlNode::Seq(s.depends_on.iter().map(|d| YamlNode::str(d)).collect()),
                    ));
                }

                if !s.env.is_empty() {
                    let env_entries: Vec<YamlEntry> = s
                        .env
                        .iter()
                        .map(|(k, v)| YamlEntry::new(k, YamlNode::str(v)))
                        .collect();
                    step_entries.push(YamlEntry::new("env", YamlNode::Map(env_entries)));
                }

                YamlNode::Map(step_entries)
            })
            .collect();

        entries.push(YamlEntry::new("steps", YamlNode::Seq(steps)));

        // Multi-flow mode — emit a `flows:` map alongside the legacy
        // single-flow entries. Consumers looking for the fleet-tool
        // schema read `flows:`; legacy consumers read `name:/steps:`.
        if !self.flows.is_empty() {
            let flow_map_entries: Vec<YamlEntry> = self
                .flows
                .iter()
                .map(|f| YamlEntry::new(&f.name, build_flow_yaml(f)))
                .collect();
            entries.push(YamlEntry::new("flows", YamlNode::Map(flow_map_entries)));
        }

        YamlNode::Map(entries)
    }
}

fn build_flow_yaml(flow: &FleetFlow) -> YamlNode {
    let mut entries = Vec::new();
    if let Some(ref desc) = flow.description {
        entries.push(YamlEntry::new("description", YamlNode::str(desc)));
    }
    let steps: Vec<YamlNode> = flow
        .steps
        .iter()
        .map(|s| {
            let mut step_entries = vec![
                YamlEntry::new("id", YamlNode::str(&s.id)),
                YamlEntry::new("action", action_to_yaml(&s.action)),
            ];
            if !s.depends_on.is_empty() {
                step_entries.push(YamlEntry::new(
                    "depends_on",
                    YamlNode::Seq(s.depends_on.iter().map(|d| YamlNode::str(d)).collect()),
                ));
            }
            if !s.env.is_empty() {
                let env_entries: Vec<YamlEntry> = s
                    .env
                    .iter()
                    .map(|(k, v)| YamlEntry::new(k, YamlNode::str(v)))
                    .collect();
                step_entries.push(YamlEntry::new("env", YamlNode::Map(env_entries)));
            }
            YamlNode::Map(step_entries)
        })
        .collect();
    entries.push(YamlEntry::new("steps", YamlNode::Seq(steps)));
    YamlNode::Map(entries)
}

fn action_to_yaml(action: &FleetAction) -> YamlNode {
    match action {
        FleetAction::Shell { command } => YamlNode::Map(vec![
            YamlEntry::new("type", YamlNode::str("shell")),
            YamlEntry::new("command", YamlNode::str(command)),
        ]),
        FleetAction::PangeaOp { op } => YamlNode::Map(vec![
            YamlEntry::new("type", YamlNode::str("pangea-op")),
            YamlEntry::new("op", YamlNode::str(op)),
        ]),
        FleetAction::AwsCli { service, subcommand, args } => {
            let args_yaml: Vec<YamlEntry> = args
                .iter()
                .map(|(k, v)| YamlEntry::new(k, YamlNode::str(v)))
                .collect();
            YamlNode::Map(vec![
                YamlEntry::new("type", YamlNode::str("aws-cli")),
                YamlEntry::new("service", YamlNode::str(service)),
                YamlEntry::new("subcommand", YamlNode::str(subcommand)),
                YamlEntry::new("args", YamlNode::Map(args_yaml)),
            ])
        }
        FleetAction::PackerBuild { packer_json, vars } => {
            let vars_yaml: Vec<YamlEntry> = vars
                .iter()
                .map(|(k, v)| YamlEntry::new(k, YamlNode::str(v)))
                .collect();
            YamlNode::Map(vec![
                YamlEntry::new("type", YamlNode::str("packer-build")),
                YamlEntry::new("packer_json", YamlNode::str(packer_json)),
                YamlEntry::new("vars", YamlNode::Map(vars_yaml)),
            ])
        }
        FleetAction::SubFlow { flow, params } => {
            let params_yaml: Vec<YamlEntry> = params
                .iter()
                .map(|(k, v)| YamlEntry::new(k, YamlNode::str(v)))
                .collect();
            YamlNode::Map(vec![
                YamlEntry::new("type", YamlNode::str("sub-flow")),
                YamlEntry::new("flow", YamlNode::str(flow)),
                YamlEntry::new("params", YamlNode::Map(params_yaml)),
            ])
        }
    }
}

// ── ShikumiConfigBuilder ────────────────────────────────────────────

/// Build a structurally correct shikumi config YAML.
pub struct ShikumiConfigBuilder {
    sections: Vec<YamlEntry>,
}

impl ShikumiConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    #[must_use]
    pub fn section(mut self, key: &str, value: YamlNode) -> Self {
        self.sections.push(YamlEntry::new(key, value));
        self
    }

    #[must_use]
    pub fn string(mut self, key: &str, value: &str) -> Self {
        self.sections.push(YamlEntry::new(key, YamlNode::str(value)));
        self
    }

    #[must_use]
    pub fn int(mut self, key: &str, value: i64) -> Self {
        self.sections.push(YamlEntry::new(key, YamlNode::Int(value)));
        self
    }

    #[must_use]
    pub fn bool(mut self, key: &str, value: bool) -> Self {
        self.sections
            .push(YamlEntry::new(key, YamlNode::Bool(value)));
        self
    }

    #[must_use]
    pub fn build(self) -> YamlNode {
        YamlNode::Map(self.sections)
    }
}

impl Default for ShikumiConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::emit_file;

    #[test]
    fn fleet_has_name() {
        let fleet = FleetBuilder::new("deploy-quero").build();
        let out = emit_file(&fleet);
        assert!(out.contains("name: deploy-quero"));
    }

    #[test]
    fn fleet_has_steps() {
        let fleet = FleetBuilder::new("test")
            .step("step1", "quero-dns", "apply", vec![], vec![])
            .build();
        let out = emit_file(&fleet);
        assert!(out.contains("steps:"));
        assert!(out.contains("step1"));
        assert!(out.contains("quero-dns"));
    }

    #[test]
    fn fleet_step_depends_on() {
        let fleet = FleetBuilder::new("test")
            .step("step1", "ws1", "apply", vec![], vec![])
            .step("step2", "ws2", "apply", vec!["step1"], vec![])
            .build();
        let out = emit_file(&fleet);
        assert!(out.contains("depends_on:"));
    }

    #[test]
    fn fleet_step_env() {
        let fleet = FleetBuilder::new("test")
            .step(
                "dns",
                "quero-dns",
                "apply",
                vec![],
                vec![("DOMAIN", "quero.lol")],
            )
            .build();
        let out = emit_file(&fleet);
        assert!(out.contains("DOMAIN"));
        assert!(out.contains("quero.lol"));
    }

    #[test]
    fn shikumi_config_simple() {
        let config = ShikumiConfigBuilder::new()
            .string("domain", "quero.lol")
            .int("port", 8080)
            .bool("enable_cache", true)
            .build();
        let out = emit_file(&config);
        assert!(out.contains("domain: quero.lol"));
        assert!(out.contains("port: 8080"));
        assert!(out.contains("enable_cache: true"));
    }

    #[test]
    fn shikumi_config_nested() {
        let config = ShikumiConfigBuilder::new()
            .section(
                "server",
                YamlNode::map(vec![
                    ("host", YamlNode::str("0.0.0.0")),
                    ("port", YamlNode::Int(3000)),
                ]),
            )
            .build();
        let out = emit_file(&config);
        assert!(out.contains("server:"));
        assert!(out.contains("host: 0.0.0.0"));
        assert!(out.contains("port: 3000"));
    }
}
