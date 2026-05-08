//! Typed DAG + flow-of-flows expansion for fleet orchestration.
//!
//! Layered on top of [`FleetFlow`]: adds explicit graph structure,
//! cycle detection, and recursive SubFlow expansion. The surface-level
//! FleetFlow API stays a flat list of steps matching the YAML schema;
//! this module turns that into a validated, traversable graph.
//!
//! ## Why typed DAG?
//!
//! The fleet tool consumes a list of steps with `depends_on: [...]`
//! and builds the DAG at runtime. A synthesis-time DAG catches three
//! classes of bugs before any deploy touches AWS:
//!
//! 1. **Missing dependencies** — `depends_on: [foo]` when there's no
//!    step named `foo`.
//! 2. **Cycles** — `a→b, b→c, c→a` — deadlocks fleet execution.
//! 3. **Unresolved SubFlow references** — `sub-flow: { flow: bar }`
//!    when the registry doesn't contain a flow named `bar`.
//!
//! ## Flow-of-flows composition
//!
//! [`FlowRegistry`] holds every known [`FleetFlow`]. [`expand_flow`]
//! walks a flow and replaces each `SubFlow { flow, params }` step with
//! the referenced flow's steps inline — applying an id prefix so steps
//! don't collide across expansions, and rewriting `depends_on` edges
//! to track the renames. SubFlow within SubFlow recurses; cycles are
//! detected during expansion.
//!
//! ## Polyglot parser sketch
//!
//! This module owns the DAG; the parser layer (yaml, nix, lisp)
//! produces `Vec<FleetFlow>` which the registry ingests. YAML parsing
//! already works via serde on the fleet-tool schema. Nix/lisp parsers
//! land later; same target type = same proofs apply.

use crate::builders::{FleetAction, FleetFlow, FleetFlowStep};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Registry of known flows. Keyed by flow name.
#[derive(Debug, Default, Clone)]
pub struct FlowRegistry {
    flows: BTreeMap<String, FleetFlow>,
}

impl FlowRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a flow. Overwrites any existing entry with the same name.
    pub fn register(&mut self, flow: FleetFlow) {
        self.flows.insert(flow.name.clone(), flow);
    }

    /// Look up a flow by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&FleetFlow> {
        self.flows.get(name)
    }

    /// Enumerate every registered flow name (sorted).
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.flows.keys().map(String::as_str)
    }
}

/// Errors from DAG validation or flow expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowError {
    /// A step's `depends_on` references an id that doesn't exist.
    MissingDependency { step_id: String, missing: String },
    /// The step graph contains a cycle.
    Cycle { path: Vec<String> },
    /// SubFlow step references a flow the registry doesn't have.
    UnknownSubFlow { step_id: String, flow: String },
    /// SubFlow expansion recursion exceeded the safety cap (likely
    /// mutual recursion through the registry).
    ExpansionTooDeep { flow: String, depth: usize },
    /// Two flows or steps share an id after expansion — a bug in the
    /// expansion renaming or the caller's inputs.
    DuplicateStepId { id: String },
}

/// Returns `Ok(())` when every step's `depends_on` references an
/// existing step id. First failure wins.
pub fn validate_dependencies(flow: &FleetFlow) -> Result<(), FlowError> {
    let ids: BTreeSet<&str> = flow.steps.iter().map(|s| s.id.as_str()).collect();
    for step in &flow.steps {
        for dep in &step.depends_on {
            if !ids.contains(dep.as_str()) {
                return Err(FlowError::MissingDependency {
                    step_id: step.id.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Topological sort. Returns a Kahn-ordered list of step ids.
/// Cycles surface as [`FlowError::Cycle`].
pub fn topological_sort(flow: &FleetFlow) -> Result<Vec<String>, FlowError> {
    validate_dependencies(flow)?;

    let mut in_degree: BTreeMap<&str, usize> = flow
        .steps
        .iter()
        .map(|s| (s.id.as_str(), 0usize))
        .collect();
    for step in &flow.steps {
        for dep in &step.depends_on {
            *in_degree.entry(step.id.as_str()).or_insert(0) += 0;
            *in_degree.entry(dep.as_str()).or_insert(0) += 0;
            *in_degree.entry(step.id.as_str()).or_insert(0) += 1;
        }
    }
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter_map(|(k, d)| if *d == 0 { Some(*k) } else { None })
        .collect();
    let mut order: Vec<String> = Vec::with_capacity(flow.steps.len());
    while let Some(id) = queue.pop_front() {
        order.push(id.to_string());
        for step in &flow.steps {
            if step.depends_on.iter().any(|d| d == id) {
                if let Some(d) = in_degree.get_mut(step.id.as_str()) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push_back(step.id.as_str());
                    }
                }
            }
        }
    }
    if order.len() == flow.steps.len() {
        Ok(order)
    } else {
        let in_cycle: Vec<String> = flow
            .steps
            .iter()
            .filter_map(|s| {
                let d = *in_degree.get(s.id.as_str()).unwrap_or(&0);
                if d > 0 { Some(s.id.clone()) } else { None }
            })
            .collect();
        Err(FlowError::Cycle { path: in_cycle })
    }
}

/// Recursive cap for SubFlow expansion. Prevents runaway recursion when
/// the registry is self-referential. 16 is plenty for realistic
/// hierarchies — the deepest we've observed is 3 (platform → layer → op).
const MAX_EXPANSION_DEPTH: usize = 16;

/// Expand every `SubFlow` step in `flow` using `registry`, producing a
/// single flat flow whose steps are either leaf actions or still-
/// unresolved SubFlows (in which case `strict = true` surfaces the
/// error; `strict = false` leaves them intact).
///
/// Expansion is deterministic: each SubFlow's steps are inserted at the
/// SubFlow step's position, with their ids prefixed by `<subflow_id>.`.
/// A step that previously `depends_on: [subflow_id]` is rewritten to
/// depend on the last step of the expanded sub-flow. Inner depends_on
/// edges get the same prefix.
pub fn expand_flow(
    flow: &FleetFlow,
    registry: &FlowRegistry,
    strict: bool,
) -> Result<FleetFlow, FlowError> {
    expand_recursive(flow, registry, strict, 0)
}

fn expand_recursive(
    flow: &FleetFlow,
    registry: &FlowRegistry,
    strict: bool,
    depth: usize,
) -> Result<FleetFlow, FlowError> {
    if depth > MAX_EXPANSION_DEPTH {
        return Err(FlowError::ExpansionTooDeep {
            flow: flow.name.clone(),
            depth,
        });
    }

    let mut out_steps: Vec<FleetFlowStep> = Vec::with_capacity(flow.steps.len());
    // Map from original step id → the id of its expansion's LAST step
    // (for rewriting downstream depends_on). For non-SubFlow steps
    // this is just the id itself.
    let mut id_tail: BTreeMap<String, String> = BTreeMap::new();

    for step in &flow.steps {
        match &step.action {
            FleetAction::SubFlow { flow: subflow_name, params: _params } => {
                let Some(sub) = registry.get(subflow_name) else {
                    if strict {
                        return Err(FlowError::UnknownSubFlow {
                            step_id: step.id.clone(),
                            flow: subflow_name.clone(),
                        });
                    }
                    out_steps.push(step.clone());
                    id_tail.insert(step.id.clone(), step.id.clone());
                    continue;
                };
                let expanded = expand_recursive(sub, registry, strict, depth + 1)?;
                let prefix = format!("{}.", step.id);
                // Rewire the parent's depends_on (the SubFlow's own
                // depends_on applies to the FIRST expanded step only;
                // inner edges keep their relative structure).
                let mut last_id: Option<String> = None;
                for (idx, inner) in expanded.steps.iter().enumerate() {
                    let new_id = format!("{}{}", prefix, inner.id);
                    let mut new_deps: Vec<String> = inner
                        .depends_on
                        .iter()
                        .map(|d| format!("{}{}", prefix, d))
                        .collect();
                    if idx == 0 {
                        // Root of the expansion inherits the SubFlow
                        // step's external dependencies.
                        new_deps.extend(
                            step.depends_on
                                .iter()
                                .filter_map(|d| id_tail.get(d).cloned()),
                        );
                    }
                    out_steps.push(FleetFlowStep {
                        id: new_id.clone(),
                        action: inner.action.clone(),
                        depends_on: new_deps,
                        env: inner.env.clone(),
                    });
                    last_id = Some(new_id);
                }
                if let Some(tail) = last_id {
                    id_tail.insert(step.id.clone(), tail);
                }
            }
            _ => {
                // Non-SubFlow — rewrite depends_on through id_tail so
                // references to prior SubFlow steps pick up the right
                // expansion tail.
                let new_deps: Vec<String> = step
                    .depends_on
                    .iter()
                    .map(|d| id_tail.get(d).cloned().unwrap_or_else(|| d.clone()))
                    .collect();
                out_steps.push(FleetFlowStep {
                    id: step.id.clone(),
                    action: step.action.clone(),
                    depends_on: new_deps,
                    env: step.env.clone(),
                });
                id_tail.insert(step.id.clone(), step.id.clone());
            }
        }
    }

    // Paranoia: expanded step ids must be unique.
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for s in &out_steps {
        if !seen.insert(s.id.as_str()) {
            return Err(FlowError::DuplicateStepId { id: s.id.clone() });
        }
    }

    Ok(FleetFlow {
        name: flow.name.clone(),
        description: flow.description.clone(),
        steps: out_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::FleetAction;

    fn shell_step(id: &str, deps: Vec<&str>) -> FleetFlowStep {
        FleetFlowStep {
            id: id.into(),
            action: FleetAction::Shell { command: format!("echo {id}") },
            depends_on: deps.into_iter().map(String::from).collect(),
            env: vec![],
        }
    }

    fn sub_step(id: &str, target: &str, deps: Vec<&str>) -> FleetFlowStep {
        FleetFlowStep {
            id: id.into(),
            action: FleetAction::SubFlow {
                flow: target.into(),
                params: vec![],
            },
            depends_on: deps.into_iter().map(String::from).collect(),
            env: vec![],
        }
    }

    #[test]
    fn validate_dependencies_accepts_valid_edges() {
        let flow = FleetFlow {
            name: "ok".into(),
            description: None,
            steps: vec![shell_step("a", vec![]), shell_step("b", vec!["a"])],
        };
        assert_eq!(validate_dependencies(&flow), Ok(()));
    }

    #[test]
    fn validate_dependencies_rejects_missing() {
        let flow = FleetFlow {
            name: "bad".into(),
            description: None,
            steps: vec![shell_step("b", vec!["a"])],
        };
        assert_eq!(
            validate_dependencies(&flow),
            Err(FlowError::MissingDependency {
                step_id: "b".into(),
                missing: "a".into(),
            })
        );
    }

    #[test]
    fn topological_sort_produces_linear_order() {
        let flow = FleetFlow {
            name: "chain".into(),
            description: None,
            steps: vec![
                shell_step("c", vec!["b"]),
                shell_step("a", vec![]),
                shell_step("b", vec!["a"]),
            ],
        };
        let order = topological_sort(&flow).expect("sort");
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn topological_sort_detects_cycle() {
        let flow = FleetFlow {
            name: "loop".into(),
            description: None,
            steps: vec![
                shell_step("a", vec!["b"]),
                shell_step("b", vec!["a"]),
            ],
        };
        let result = topological_sort(&flow);
        assert!(matches!(result, Err(FlowError::Cycle { .. })));
    }

    #[test]
    fn expand_flow_inlines_subflow_steps() {
        let mut reg = FlowRegistry::new();
        reg.register(FleetFlow {
            name: "deploy".into(),
            description: None,
            steps: vec![
                shell_step("synth", vec![]),
                shell_step("apply", vec!["synth"]),
            ],
        });
        let outer = FleetFlow {
            name: "release".into(),
            description: None,
            steps: vec![
                shell_step("prove", vec![]),
                sub_step("roll", "deploy", vec!["prove"]),
                shell_step("verify", vec!["roll"]),
            ],
        };
        let expanded = expand_flow(&outer, &reg, true).expect("expansion");
        let ids: Vec<&str> = expanded.steps.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["prove", "roll.synth", "roll.apply", "verify"]);
        // The expansion's first step inherits the SubFlow's external deps.
        let synth = expanded
            .steps
            .iter()
            .find(|s| s.id == "roll.synth")
            .unwrap();
        assert!(synth.depends_on.contains(&"prove".into()));
        // verify now depends on the LAST step of the expansion.
        let verify = expanded
            .steps
            .iter()
            .find(|s| s.id == "verify")
            .unwrap();
        assert_eq!(verify.depends_on, vec!["roll.apply"]);
    }

    #[test]
    fn expand_flow_strict_fails_on_unknown_subflow() {
        let reg = FlowRegistry::new();
        let outer = FleetFlow {
            name: "x".into(),
            description: None,
            steps: vec![sub_step("step", "missing", vec![])],
        };
        let result = expand_flow(&outer, &reg, true);
        assert!(matches!(result, Err(FlowError::UnknownSubFlow { .. })));
    }

    #[test]
    fn expand_flow_non_strict_keeps_unresolved_subflow() {
        let reg = FlowRegistry::new();
        let outer = FleetFlow {
            name: "x".into(),
            description: None,
            steps: vec![sub_step("step", "missing", vec![])],
        };
        let expanded = expand_flow(&outer, &reg, false).expect("non-strict keeps subflow");
        assert_eq!(expanded.steps.len(), 1);
        assert!(matches!(
            expanded.steps[0].action,
            FleetAction::SubFlow { .. }
        ));
    }

    #[test]
    fn expand_flow_recurses_subflow_within_subflow() {
        let mut reg = FlowRegistry::new();
        reg.register(FleetFlow {
            name: "inner".into(),
            description: None,
            steps: vec![shell_step("leaf", vec![])],
        });
        reg.register(FleetFlow {
            name: "middle".into(),
            description: None,
            steps: vec![sub_step("nest", "inner", vec![])],
        });
        let outer = FleetFlow {
            name: "outer".into(),
            description: None,
            steps: vec![sub_step("step", "middle", vec![])],
        };
        let expanded = expand_flow(&outer, &reg, true).expect("nested expansion");
        let ids: Vec<&str> = expanded.steps.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["step.nest.leaf"]);
    }

    #[test]
    fn expand_flow_detects_infinite_recursion() {
        // Two flows that reference each other. Without the depth cap
        // this loops forever.
        let mut reg = FlowRegistry::new();
        reg.register(FleetFlow {
            name: "a".into(),
            description: None,
            steps: vec![sub_step("step", "b", vec![])],
        });
        reg.register(FleetFlow {
            name: "b".into(),
            description: None,
            steps: vec![sub_step("step", "a", vec![])],
        });
        let outer = FleetFlow {
            name: "entry".into(),
            description: None,
            steps: vec![sub_step("start", "a", vec![])],
        };
        let result = expand_flow(&outer, &reg, true);
        assert!(matches!(result, Err(FlowError::ExpansionTooDeep { .. })));
    }

    #[test]
    fn expanded_flow_topologically_sorts() {
        // Flow-of-flows should still be a valid DAG after expansion.
        let mut reg = FlowRegistry::new();
        reg.register(FleetFlow {
            name: "pipeline".into(),
            description: None,
            steps: vec![
                shell_step("a", vec![]),
                shell_step("b", vec!["a"]),
                shell_step("c", vec!["b"]),
            ],
        });
        let outer = FleetFlow {
            name: "root".into(),
            description: None,
            steps: vec![
                shell_step("pre", vec![]),
                sub_step("inner", "pipeline", vec!["pre"]),
                shell_step("post", vec!["inner"]),
            ],
        };
        let expanded = expand_flow(&outer, &reg, true).expect("expansion");
        let order = topological_sort(&expanded).expect("sort expanded flow");
        // `pre` before `inner.a`, `inner.c` before `post`.
        let pre = order.iter().position(|x| x == "pre").unwrap();
        let inner_a = order.iter().position(|x| x == "inner.a").unwrap();
        let inner_c = order.iter().position(|x| x == "inner.c").unwrap();
        let post = order.iter().position(|x| x == "post").unwrap();
        assert!(pre < inner_a);
        assert!(inner_a < inner_c);
        assert!(inner_c < post);
    }
}
