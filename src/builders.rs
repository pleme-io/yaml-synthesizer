use crate::node::{YamlEntry, YamlNode};

// ── FleetBuilder ──────────────────────────────────────────────────���─

/// Build a structurally correct fleet.yaml for Pangea deployment flows.
pub struct FleetBuilder {
    name: String,
    description: Option<String>,
    steps: Vec<FleetStep>,
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
        }
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

        YamlNode::Map(entries)
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
