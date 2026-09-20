use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskDefinition {
    pub(crate) id: String,
    pub(crate) function: String,
    pub(crate) args: HashMap<String, Argument>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Argument {
    Value(Value),
    FromTask(String),
}

pub(crate) struct TaskGraph {
    tasks: HashMap<String, TaskDefinition>,
    dependencies: HashMap<String, HashSet<String>>,
    dependents: HashMap<String, HashSet<String>>,
    outputs: Vec<String>,
}

impl TaskGraph {
    // Reject duplicate task IDs and build indexes; validate other graph rules separately.
    pub(crate) fn new(definitions: Vec<TaskDefinition>, outputs: Vec<String>) -> Result<Self, String> {
		let mut tasks = HashMap::new();
		
		for task in definitions {
			if tasks.contains_key(&task.id) {
				return Err(format!("Duplicate task id: {}", task.id.clone()));
			}
			tasks.insert(task.id.clone(), task);
		}

        let mut graph = Self {
            tasks,
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            outputs,
        };

        for (id, task) in &graph.tasks {
            let prerequisites = graph.dependencies.entry(id.clone()).or_default();
            graph.dependents.entry(id.clone()).or_default();
            for argument in task.args.values() {
                if let Argument::FromTask(upstream) = argument {
                    prerequisites.insert(upstream.clone());
                    graph
                        .dependents
                        .entry(upstream.clone())
                        .or_default()
                        .insert(id.clone());
                }
            }
        }
        Ok(graph)
    }

    pub(crate) fn tasks(&self) -> &HashMap<String, TaskDefinition> {
        &self.tasks
    }

    pub(crate) fn outputs(&self) -> &[String] {
        &self.outputs
    }

	/// Whether a task graph is safe to accept
    pub(crate) fn validate(&self) -> Result<(), String> {
		// at least one task and one requested output
		if self.tasks.len() == 0 {
			return Err(String::from("Need at least one task"));
		}
		
		if self.outputs.len() == 0 {
			return Err(String::from("Need at least one requested output"));
		}
		
		// non empty task ids and function names
		if self.tasks.iter().any(|(task_id, task)| {
			task_id.trim().is_empty() || task.function.trim().is_empty()
			}) {
				return Err(String::from("Task IDs and function names must be nonempty"));
			}
		
		// every from_task references an existing task
		for (task_id, task) in &self.tasks {
		    for (arg_name, argument) in &task.args {
		        if let Argument::FromTask(upstream_id) = argument {
		            if !self.tasks.contains_key(upstream_id) {
		                return Err(format!(
		                    "Task '{task_id}', argument '{arg_name}': \
		                     referenced task '{upstream_id}' does not exist."
		                ));
		            }
		        }
		    }
		}
		
		// every requested output references an existing task
		for output_id in &self.outputs {
			if !self.tasks.contains_key(output_id) {
				return Err(format!(
					"Requested output '{output_id}' does not reference an existing task"));
			}
		}
		
		// no cycles
		if self.detect_cycle() {
			return Err("DAG cannot contain a cycle".to_string());
		}
		
		Ok(())
    }

    /// Return true when Kahn's algorithm cannot process every task.
    /// Call only after validating that all task references exist.
    pub(crate) fn detect_cycle(&self) -> bool {
        let mut remaining: HashMap<&str, usize> = self
            .dependencies
            .iter()
            .map(|(task_id, prerequisites)| (task_id.as_str(), prerequisites.len()))
            .collect();

        let mut queue: VecDeque<&str> = remaining
            .iter()
            .filter_map(|(&task_id, &count)| (count == 0).then_some(task_id))
            .collect();
        let mut processed_count = 0;

        while let Some(task_id) = queue.pop_front() {
            processed_count += 1;

            for dependent in &self.dependents[task_id] {
                let count = remaining
                    .get_mut(dependent.as_str())
                    .expect("dependents are indexed tasks");
                *count -= 1;

                if *count == 0 {
                    queue.push_back(dependent.as_str());
                }
            }
        }

        processed_count < self.tasks.len()
    }

    /// Draw each task once, with dependency arrows flowing top to bottom.
    pub(crate) fn draw_graph(&self) -> String {
        use mermaid_text::{Direction, Edge, Graph, Node, NodeShape};

        let ids: BTreeSet<&str> = self
            .tasks
            .keys()
            .chain(self.dependents.keys())
            .chain(self.outputs.iter())
            .map(String::as_str)
            .collect();
        if ids.is_empty() {
            return "Task graph (empty)\n".into();
        }

        let outputs: HashSet<&str> = self.outputs.iter().map(String::as_str).collect();
        let mut diagram = Graph::new(Direction::TopToBottom);
        for id in &ids {
            let description = self.tasks.get(*id).map_or_else(
                || "[undefined task]".to_owned(),
                |task| task.function.escape_debug().to_string(),
            );
            let mut label = format!("{}\n{description}", id.escape_debug());
            if outputs.contains(id) {
                label.push_str("\n[output]");
            }
            // Use the typed renderer model: labels never become Mermaid syntax.
            diagram
                .nodes
                .push(Node::new(*id, label, NodeShape::Rounded));
        }
        let mut edges: Vec<_> = self
            .dependencies
            .iter()
            .flat_map(|(id, dependencies)| dependencies.iter().map(move |upstream| (upstream, id)))
            .collect();
        edges.sort_unstable();
        for (upstream, downstream) in edges {
            diagram.edges.push(Edge::new(upstream, downstream, None));
        }

        let layout = mermaid_text::layout::sugiyama_layout(
            &diagram,
            &mermaid_text::layout::LayoutConfig {
                layer_gap: 3,
                node_gap: 4,
                ..Default::default()
            },
        );
        let drawing = mermaid_text::render::render(&diagram, &layout.positions, &[]);
        format!("Task graph (top to bottom)\n{drawing}\n")
    }
	
	pub(crate) fn dependencies(&self, task_id: &str) -> Option<&HashSet<String>> {
		self.dependencies.get(task_id)
	}
	pub(crate) fn dependents(&self, task_id: &str) -> Option<&HashSet<String>> {
		self.dependents.get(task_id)
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_graph(tasks: Vec<TaskDefinition>, outputs: Vec<String>) -> TaskGraph {
        TaskGraph::new(tasks, outputs)
            .unwrap_or_else(|error| panic!("Unexpected construction error: {error}"))
    }

    fn task(id: &str, upstream: &[&str]) -> TaskDefinition {
        TaskDefinition {
            id: id.into(),
            function: "example".into(),
            args: upstream
                .iter()
                .enumerate()
                .map(|(i, id)| (format!("arg_{i}"), Argument::FromTask((*id).into())))
                .collect(),
        }
    }

    #[test]
    fn builds_both_indexes_and_deduplicates_dependencies() {
        let graph = build_graph(
            vec![
                task("total", &["left", "left", "right"]),
                task("left", &[]),
                task("right", &[]),
            ],
            vec!["total".into()],
        );
        assert_eq!(
            graph.dependencies["total"],
            HashSet::from(["left".into(), "right".into()])
        );
        assert!(graph.dependencies["left"].is_empty());
        assert_eq!(graph.dependents["left"], HashSet::from(["total".into()]));
        assert!(graph.dependents["total"].is_empty());
    }

    #[test]
    fn literal_objects_do_not_create_dependencies() {
        let definition = serde_json::from_value(json!({
            "id": "a", "function": "example",
            "args": {"payload": {"value": {"from_task": "not-a-reference"}}}
        }))
        .unwrap();
        let graph = build_graph(vec![definition], vec!["a".into()]);
        assert!(graph.dependencies["a"].is_empty());
    }

    #[test]
    fn drawing_shows_fan_out_shared_nodes_and_outputs_in_stable_order() {
        let graph = build_graph(
            vec![
                task("root", &[]),
                task("right", &["root"]),
                task("left", &["root"]),
                task("total", &["left", "right"]),
            ],
            vec!["total".into()],
        );
        let drawing = graph.draw_graph();
        println!("{drawing}");
        for id in ["root", "left", "right", "total"] {
            assert_eq!(drawing.matches(id).count(), 1);
        }
        let row = |label: &str| {
            drawing
                .lines()
                .position(|line| line.contains(label))
                .unwrap()
        };
        assert!(row("root") < row("left"));
        assert_eq!(row("left"), row("right"));
        assert!(row("left") < row("total"));
        assert!(drawing.contains("[output]"));
        assert!(drawing.contains('╭'));
        assert!(drawing.contains('▾'));
        assert_eq!(drawing, graph.draw_graph());
    }

    #[test]
    fn drawing_handles_cycles_missing_nodes_and_disconnected_components() {
        let graph = build_graph(
            vec![
                task("a", &["b"]),
                task("b", &["a"]),
                task("island", &[]),
                task("consumer", &["missing"]),
            ],
            vec!["unknown-output".into()],
        );
        let drawing = graph.draw_graph();
        for label in ["island", "missing", "consumer", "unknown-output"] {
            assert_eq!(drawing.matches(label).count(), 1);
        }
        assert_eq!(drawing.matches("[undefined task]").count(), 2);
        assert!(drawing.contains("[output]"));
        assert_eq!(
            build_graph(vec![], vec![]).draw_graph(),
            "Task graph (empty)\n"
        );
    }

    #[test]
    fn drawing_escapes_control_characters_in_labels() {
        let graph = build_graph(vec![task("line\nbreak", &[])], vec![]);
        assert!(graph.draw_graph().contains("line\\nbreak"));
    }

    #[test]
    fn cycle_detection_accepts_acyclic_graphs_without_mutating_them() {
        for tasks in [
            vec![],
            vec![task("a", &[])],
            vec![task("a", &[]), task("b", &["a"]), task("c", &["b"])],
            vec![
                task("root", &[]),
                task("left", &["root"]),
                task("right", &["root"]),
                task("total", &["left", "left", "right"]),
            ],
            vec![task("a", &[]), task("b", &[])],
        ] {
            let graph = build_graph(tasks, vec![]);
            let dependencies = graph.dependencies.clone();
            let dependents = graph.dependents.clone();
            assert!(!graph.detect_cycle());
            assert!(!graph.detect_cycle());
            assert_eq!(graph.dependencies, dependencies);
            assert_eq!(graph.dependents, dependents);
        }
    }

    #[test]
    fn cycle_detection_finds_self_loops_and_disconnected_cycles() {
        for tasks in [
            vec![task("a", &["a"])],
            vec![task("a", &["b"]), task("b", &["a"])],
            vec![task("a", &["c"]), task("b", &["a"]), task("c", &["b"])],
            vec![
                task("independent", &[]),
                task("a", &["b"]),
                task("b", &["a"]),
            ],
            vec![
                task("root", &[]),
                task("a", &["root", "b"]),
                task("b", &["a"]),
            ],
        ] {
            assert!(build_graph(tasks, vec![]).detect_cycle());
        }
    }

    #[test]
    fn validation_accepts_shared_dependencies_and_multiple_outputs() {
        let graph = build_graph(
            vec![
                task("root", &[]),
                task("a", &["root", "root"]),
                task("b", &["root"]),
                task("total", &["a", "b"]),
            ],
            vec!["a".into(), "total".into()],
        );
        let dependencies = graph.dependencies.clone();
        let dependents = graph.dependents.clone();
        assert_eq!(graph.validate(), Ok(()));
        assert_eq!(graph.dependencies, dependencies);
        assert_eq!(graph.dependents, dependents);
    }

    #[test]
    fn validation_rejects_empty_tasks_or_outputs() {
        assert!(build_graph(vec![], vec!["a".into()]).validate().is_err());
        assert!(
            build_graph(vec![task("a", &[])], vec![])
                .validate()
                .is_err()
        );
    }

    #[test]
    fn validation_rejects_blank_task_ids_and_function_names() {
        for blank in ["", " ", "\t\n"] {
            assert!(
                build_graph(vec![task(blank, &[])], vec![blank.into()])
                    .validate()
                    .is_err()
            );
            let mut definition = task("a", &[]);
            definition.function = blank.into();
            assert!(
                build_graph(vec![definition], vec!["a".into()])
                    .validate()
                    .is_err()
            );
        }
    }

    #[test]
    fn validation_reports_missing_reference_before_cycle_detection() {
        let graph = build_graph(
            vec![task("consumer", &["missing"])],
            vec!["consumer".into()],
        );
        let error = graph.validate().unwrap_err();
        assert!(error.contains("consumer"));
        assert!(error.contains("arg_0"));
        assert!(error.contains("missing"));
        assert!(!error.to_lowercase().contains("cycle"));
    }

    #[test]
    fn validation_checks_every_requested_output() {
        let graph = build_graph(vec![task("a", &[])], vec!["a".into(), "missing".into()]);
        assert!(graph.validate().unwrap_err().contains("missing"));
    }

    #[test]
    fn validation_rejects_self_loops_and_disconnected_cycles() {
        for definitions in [
            vec![task("output", &["output"])],
            vec![task("output", &[]), task("a", &["b"]), task("b", &["a"])],
        ] {
            let graph = build_graph(definitions, vec!["output".into()]);
            assert!(
                graph
                    .validate()
                    .unwrap_err()
                    .to_lowercase()
                    .contains("cycle")
            );
        }
    }

    #[test]
    fn validation_does_not_treat_literal_objects_as_references() {
        let definition = serde_json::from_value(json!({
            "id": "a", "function": "example",
            "args": {"data": {"value": {"from_task": "missing"}}}
        }))
        .unwrap();
        assert_eq!(
            build_graph(vec![definition], vec!["a".into()]).validate(),
            Ok(())
        );
    }

    #[test]
    fn construction_rejects_duplicate_task_ids() {
        let result = TaskGraph::new(
            vec![task("duplicate", &[]), task("duplicate", &[])],
            vec!["duplicate".into()],
        );
        assert!(
            result.is_err(),
            "Duplicate IDs must be rejected before insertion into the map"
        );
    }
}
