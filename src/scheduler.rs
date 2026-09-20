use std::collections::{HashMap, VecDeque};
use crate::graph::TaskGraph;

enum TaskStatus {
	Pending,
	Ready,
	Running,
	Succeeded,
	Failed,
}

struct TaskExecution {
	status: TaskStatus,
	remaining_dependencies: usize,
}

pub(crate) struct Scheduler {
	executions: HashMap<String, TaskExecution>,
	ready: VecDeque<String>,
	failed: bool,
}

impl Scheduler {
	// new(&TaskGraph): init statuses, counters, and ready queue
	pub fn new(graph: &TaskGraph) -> Self {
		let mut executions: HashMap<String, TaskExecution> = HashMap::new();
		let mut ready = VecDeque::new();
		
		for (task_id, _) in graph.tasks() {
			executions.insert(task_id.to_string(), TaskExecution {
				status: TaskStatus::Pending,
				remaining_dependencies: graph.dependencies(task_id).expect("Every task has a dependency entry").len(),
			});
		}
		
		for (task_id, ex) in &mut executions {
			if ex.remaining_dependencies == 0 {
				ready.push_back(task_id.clone());
				ex.status = TaskStatus::Ready;
			}
		}
		
		Self { executions, ready, failed: false }
	}
	
	// pop first ready task and mark it running
	pub fn next_task(&mut self) -> Option<String> {
		if self.failed {
			return None;
		}
		let task_id = self.ready.pop_front()?;
		
		let task_ex = self.executions
			.get_mut(&task_id)
			.expect("Every ready task has an executions entry");
			
		task_ex.status = TaskStatus::Running;
		Some(task_id)
	}
	
	// Marks the task as succeeded and reduces dependents
	pub fn task_succeeded(&mut self, graph: &TaskGraph, task_id: &str) -> Result<(), String> {
		let Some(task) = self.executions.get_mut(task_id) else {
			return Err("Task does not exist in executions".to_string());
		};
		
		match &mut task.status {
			TaskStatus::Succeeded => return Ok(()),
			TaskStatus::Running => task.status = TaskStatus::Succeeded,
			_ => return Err("Task status invalid".to_string()),
		};
		
		for t_id in graph.dependents(task_id).unwrap() {
			if let Some(t_ex) = self.executions.get_mut(t_id) {
				t_ex.remaining_dependencies -= 1;
				
				if t_ex.remaining_dependencies == 0 {
					t_ex.status = TaskStatus::Ready;
					self.ready.push_back(t_id.clone());
				}
			}
		}
		
		Ok(())
	}
	
	// Record failure and stop dispatching new work for this job.
	pub fn task_failed(&mut self, task_id: &str) -> Result<(), String> {
		if let Some(task_ex) = self.executions.get_mut(task_id) {
			match task_ex.status {
				TaskStatus::Failed => return Ok(()),
				TaskStatus::Running => {
					task_ex.status = TaskStatus::Failed;
					self.failed = true;
				},
				_ => return Err(String::from("Invalid task executions")),
			}
		} else {
			return Err("Invalid task_id, not present in executions".to_string());
		}
		
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::{BTreeMap, HashSet};
	use crate::graph::{Argument, TaskDefinition};
	use serde_json::json;

	fn build_scheduler(graph: &TaskGraph) -> Scheduler {
		Scheduler::new(graph)
	}

    fn task(id: &str, dependencies: &[&str]) -> TaskDefinition {
        TaskDefinition {
            id: id.into(),
            function: "example".into(),
            args: dependencies.iter().enumerate().map(|(index, upstream)| {
                (format!("arg_{index}"), Argument::FromTask((*upstream).into()))
            }).collect(),
        }
    }

    fn graph(tasks: Vec<TaskDefinition>, outputs: &[&str]) -> TaskGraph {
        let graph = TaskGraph::new(tasks, outputs.iter().map(|id| (*id).into()).collect())
            .expect("Fixture should construct");
        graph.validate().expect("Fixture should be valid");
        graph
    }

    fn status_name(status: &TaskStatus) -> &'static str {
        match status {
            TaskStatus::Pending => "pending",
            TaskStatus::Ready => "ready",
            TaskStatus::Running => "running",
            TaskStatus::Succeeded => "succeeded",
            TaskStatus::Failed => "failed",
        }
    }

    fn snapshot(scheduler: &Scheduler) -> (BTreeMap<String, (&'static str, usize)>, Vec<String>, bool) {
        (scheduler.executions.iter().map(|(id, execution)| {
            (id.clone(), (status_name(&execution.status), execution.remaining_dependencies))
        }).collect(), scheduler.ready.iter().cloned().collect(), scheduler.failed)
    }

    fn assert_state(scheduler: &Scheduler, id: &str, status: &str, remaining: usize) {
        let execution = &scheduler.executions[id];
        assert_eq!(status_name(&execution.status), status, "Status of {id}");
        assert_eq!(execution.remaining_dependencies, remaining, "Dependencies of {id}");
    }

    #[test]
    fn failure_stops_dispatch_without_consuming_ready_tasks() {
        let graph = graph(vec![task("a", &[]), task("b", &[])], &["a", "b"]);
        let mut scheduler = Scheduler::new(&graph);
        assert!(!scheduler.failed);
        let running = scheduler.next_task().unwrap();
        scheduler.task_failed(&running).unwrap();
        assert!(scheduler.failed);
        assert_state(&scheduler, &running, "failed", 0);
        assert_eq!(scheduler.ready.len(), 1);
        let before = snapshot(&scheduler);
        for _ in 0..3 {
            assert_eq!(scheduler.next_task(), None);
            scheduler.task_failed(&running).unwrap();
            assert_eq!(snapshot(&scheduler), before);
        }
        assert!(scheduler.task_succeeded(&graph, &running).is_err());
        assert_eq!(snapshot(&scheduler), before);
    }

    #[test]
    fn invalid_failure_reports_leave_state_unchanged() {
        let graph = graph(vec![task("root", &[]), task("child", &["root"])], &["child"]);
        let mut scheduler = Scheduler::new(&graph);
        for id in ["missing", "", "root", "child"] {
            let before = snapshot(&scheduler);
            assert!(scheduler.task_failed(id).is_err());
            assert_eq!(snapshot(&scheduler), before);
        }
        scheduler.next_task().unwrap();
        scheduler.task_succeeded(&graph, "root").unwrap();
        let before = snapshot(&scheduler);
        assert!(scheduler.task_failed("root").is_err());
        assert_eq!(snapshot(&scheduler), before);
    }

    #[test]
    fn late_success_after_failure_cannot_restart_dispatch() {
        let graph = graph(vec![
            task("a", &[]), task("b", &[]),
            task("child", &["b"]), task("join", &["a", "b"]),
        ], &["child", "join"]);
        let mut scheduler = Scheduler::new(&graph);
        scheduler.next_task().unwrap();
        scheduler.next_task().unwrap();
        scheduler.task_failed("a").unwrap();
        assert_state(&scheduler, "join", "pending", 2);
        scheduler.task_succeeded(&graph, "b").unwrap();
        assert_state(&scheduler, "b", "succeeded", 0);
        assert_state(&scheduler, "child", "ready", 0);
        assert_state(&scheduler, "join", "pending", 1);
        let before = snapshot(&scheduler);
        assert_eq!(scheduler.next_task(), None);
        scheduler.task_succeeded(&graph, "b").unwrap();
        assert_eq!(snapshot(&scheduler), before);
        assert!(scheduler.failed);
    }

    #[test]
    fn multiple_running_tasks_can_fail_without_affecting_other_jobs() {
        let graph = graph(vec![task("a", &[]), task("b", &[])], &["a", "b"]);
        let mut scheduler = Scheduler::new(&graph);
        let mut other_job = Scheduler::new(&graph);
        scheduler.next_task().unwrap();
        scheduler.next_task().unwrap();
        scheduler.task_failed("a").unwrap();
        scheduler.task_failed("b").unwrap();
        assert_state(&scheduler, "a", "failed", 0);
        assert_state(&scheduler, "b", "failed", 0);
        assert_eq!(scheduler.next_task(), None);
        assert!(!other_job.failed);
        assert!(other_job.next_task().is_some());
    }

    #[test]
    fn initialization_counts_unique_dependencies_and_ignores_literals() {
        let mut root = task("root", &[]);
        root.args.insert("literal".into(), Argument::Value(json!({"from_task": "missing"})));
        let graph = graph(vec![root, task("child", &["root", "root"])], &["child"]);
        let mut scheduler = Scheduler::new(&graph);
        assert_eq!(scheduler.executions.len(), 2);
        assert_state(&scheduler, "root", "ready", 0);
        assert_state(&scheduler, "child", "pending", 1);
        assert_eq!(scheduler.next_task().as_deref(), Some("root"));
        scheduler.task_succeeded(&graph, "root").unwrap();
        assert_state(&scheduler, "child", "ready", 0);
        assert_eq!(scheduler.next_task().as_deref(), Some("child"));
        assert_eq!(scheduler.next_task(), None);
    }

    #[test]
    fn dispatch_changes_only_one_task_and_empty_queue_preserves_state() {
        let graph = graph(vec![task("a", &[]), task("b", &[])], &["a", "b"]);
        let mut scheduler = Scheduler::new(&graph);
        // HashMap iteration does not define the initial order; dispatch must follow
        // whichever queue order initialization produced.
        let order: Vec<_> = scheduler.ready.iter().cloned().collect();
        assert_eq!(scheduler.next_task(), Some(order[0].clone()));
        assert_state(&scheduler, &order[0], "running", 0);
        assert_state(&scheduler, &order[1], "ready", 0);
        assert_eq!(scheduler.next_task(), Some(order[1].clone()));
        let before = snapshot(&scheduler);
        assert_eq!(scheduler.next_task(), None);
        assert_eq!(scheduler.next_task(), None);
        assert_eq!(snapshot(&scheduler), before);
    }

    #[test]
    fn chain_releases_only_the_immediate_dependent() {
        let graph = graph(vec![task("a", &[]), task("b", &["a"]), task("c", &["b"])], &["c"]);
        let mut scheduler = Scheduler::new(&graph);
        assert_eq!(scheduler.next_task().as_deref(), Some("a"));
        scheduler.task_succeeded(&graph, "a").unwrap();
        assert_state(&scheduler, "a", "succeeded", 0);
        assert_state(&scheduler, "b", "ready", 0);
        assert_state(&scheduler, "c", "pending", 1);
        assert_eq!(scheduler.next_task().as_deref(), Some("b"));
        scheduler.task_succeeded(&graph, "b").unwrap();
        assert_eq!(scheduler.next_task().as_deref(), Some("c"));
        scheduler.task_succeeded(&graph, "c").unwrap();
        assert!(scheduler.executions.values().all(|execution| matches!(execution.status, TaskStatus::Succeeded)));
        assert_eq!(scheduler.next_task(), None);
    }

    #[test]
    fn completed_branch_progresses_without_waiting_for_unrelated_running_task() {
        let graph = graph(vec![task("a", &[]), task("b", &[]), task("child", &["a"])], &["child", "b"]);
        let mut scheduler = Scheduler::new(&graph);
        let roots: HashSet<_> = (0..2).map(|_| scheduler.next_task().unwrap()).collect();
        assert_eq!(roots, HashSet::from(["a".into(), "b".into()]));
        scheduler.task_succeeded(&graph, "a").unwrap();
        assert_eq!(scheduler.next_task().as_deref(), Some("child"));
        assert_state(&scheduler, "b", "running", 0);
        scheduler.task_succeeded(&graph, "child").unwrap();
        scheduler.task_succeeded(&graph, "b").unwrap();
        assert_eq!(scheduler.next_task(), None);
    }

    #[test]
    fn invalid_completions_return_errors_without_changing_state() {
        let graph = graph(vec![task("root", &[]), task("child", &["root"])], &["child"]);
        let mut scheduler = Scheduler::new(&graph);
        for id in ["missing", "", "root", "child"] {
            let before = snapshot(&scheduler);
            assert!(scheduler.task_succeeded(&graph, id).is_err(), "Should reject {id:?}");
            assert_eq!(snapshot(&scheduler), before);
        }
        // Failure reporting is not implemented yet. Seed Failed solely to verify
        // that task_succeeded rejects this terminal state without releasing work.
        scheduler.executions.get_mut("root").unwrap().status = TaskStatus::Failed;
        let before = snapshot(&scheduler);
        assert!(scheduler.task_succeeded(&graph, "root").is_err());
        assert_eq!(snapshot(&scheduler), before);
    }

    #[test]
    fn duplicate_completion_is_harmless_before_and_after_dependent_dispatch() {
        let graph = graph(vec![task("root", &[]), task("child", &["root"])], &["child"]);
        let mut scheduler = Scheduler::new(&graph);
        scheduler.next_task().unwrap();
        scheduler.task_succeeded(&graph, "root").unwrap();
        let before = snapshot(&scheduler);
        for _ in 0..3 {
            scheduler.task_succeeded(&graph, "root").unwrap();
            assert_eq!(snapshot(&scheduler), before);
        }
        assert_eq!(scheduler.next_task().as_deref(), Some("child"));
        let before = snapshot(&scheduler);
        scheduler.task_succeeded(&graph, "root").unwrap();
        assert_eq!(snapshot(&scheduler), before);
        scheduler.task_succeeded(&graph, "child").unwrap();
        let before = snapshot(&scheduler);
        scheduler.task_succeeded(&graph, "root").unwrap();
        scheduler.task_succeeded(&graph, "child").unwrap();
        assert_eq!(snapshot(&scheduler), before);
    }

    #[test]
    fn separate_schedulers_do_not_share_execution_state() {
        let graph = graph(vec![task("root", &[])], &["root"]);
        let mut first = Scheduler::new(&graph);
        let mut second = Scheduler::new(&graph);
        first.next_task().unwrap();
        first.task_succeeded(&graph, "root").unwrap();
        assert_state(&second, "root", "ready", 0);
        assert_eq!(second.next_task().as_deref(), Some("root"));
        assert_state(&first, "root", "succeeded", 0);
    }

    #[test]
    fn all_four_task_dags_respect_dependencies_under_every_completion_priority() {
        let ids = ["a", "b", "c", "d"];
        let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        // All 64 forward-edge subsets, including chains, diamonds, fan-out,
        // fan-in and disconnected graphs. Try all 24 completion priorities.
        for mask in 0..(1 << edges.len()) {
            let prerequisites: Vec<Vec<usize>> = (0..4).map(|child| {
                edges.iter().enumerate().filter_map(|(bit, &(parent, target))| {
                    (target == child && mask & (1 << bit) != 0).then_some(parent)
                }).collect()
            }).collect();
            let graph = graph((0..4).map(|index| {
                task(ids[index], &prerequisites[index].iter().map(|&parent| ids[parent]).collect::<Vec<_>>())
            }).collect(), &ids);
            for a in 0..4 {
                for b in 0..4 {
                    for c in 0..4 {
                        for d in 0..4 {
                            let priority = [a, b, c, d];
                            if priority.iter().copied().collect::<HashSet<_>>().len() != 4 { continue; }
                            let mut scheduler = Scheduler::new(&graph);
                            let mut started = HashSet::new();
                            let mut completed = HashSet::new();
                            while completed.len() < 4 {
                                while let Some(id) = scheduler.next_task() {
                                    let index = ids.iter().position(|&candidate| candidate == id).unwrap();
                                    assert!(prerequisites[index].iter().all(|parent| completed.contains(parent)), "Premature dispatch: mask {mask}, task {id}");
                                    assert!(started.insert(index), "Task dispatched twice");
                                    assert_state(&scheduler, &id, "running", 0);
                                }
                                let next = priority.iter().copied().find(|index| started.contains(index) && !completed.contains(index))
                                    .expect("An unfinished DAG must make progress");
                                scheduler.task_succeeded(&graph, ids[next]).unwrap();
                                completed.insert(next);
                                let before = snapshot(&scheduler);
                                scheduler.task_succeeded(&graph, ids[next]).unwrap();
                                assert_eq!(snapshot(&scheduler), before);
                                for index in 0..4 {
                                    let remaining = prerequisites[index].iter().filter(|parent| !completed.contains(parent)).count();
                                    let status = if completed.contains(&index) { "succeeded" }
                                        else if started.contains(&index) { "running" }
                                        else if remaining == 0 { "ready" } else { "pending" };
                                    assert_state(&scheduler, ids[index], status, remaining);
                                    assert_eq!(scheduler.ready.iter().filter(|id| id.as_str() == ids[index]).count(), usize::from(status == "ready"));
                                }
                            }
                            assert_eq!(scheduler.next_task(), None);
                        }
                    }
                }
            }
        }
    }
	
	#[test]
	fn v_shape() {
		let t_1 = TaskDefinition {
		    id: "task_1".into(),
		    function: "sum_numbers".into(),
		    args: HashMap::from([
		        ("numbers".into(), Argument::Value(json!([1, 2]))),
		    ]),
		};

		let t_2 = TaskDefinition {
		    id: "task_2".into(),
		    function: "sum_numbers".into(),
		    args: HashMap::from([
		        ("numbers".into(), Argument::Value(json!([3, 4]))),
		    ]),
		};

		let t_t = TaskDefinition {
		    id: "total".into(),
		    function: "combine_sums".into(),
		    args: HashMap::from([
		        ("first".into(), Argument::FromTask("task_1".into())),
		        ("second".into(), Argument::FromTask("task_2".into())),
		    ]),
		};
		
		let graph = TaskGraph::new(
			vec![t_t, t_1, t_2],
			vec!["total".into()])
				.expect("Graph construction should succeed");
		
			graph.validate().expect("Graph should be valid");
			
		let mut scheduler = build_scheduler(&graph);
		let first = scheduler.next_task().expect("First task ready");
		let second = scheduler.next_task().expect("Second task ready");

		assert!(matches!(first.as_str(), "task_1" | "task_2"));
		assert!(matches!(second.as_str(), "task_1" | "task_2"));
		assert_ne!(first, second);
		assert_eq!(scheduler.next_task(), None);

		scheduler.task_succeeded(&graph, &first).unwrap();
		assert_eq!(scheduler.next_task(), None);

		// A duplicate completion must not release total.
		scheduler.task_succeeded(&graph, &first).unwrap();
		assert_eq!(scheduler.next_task(), None);

		scheduler.task_succeeded(&graph, &second).unwrap();
		assert_eq!(scheduler.next_task(), Some("total".into()));
		assert_eq!(scheduler.next_task(), None);

		scheduler.task_succeeded(&graph, "total").unwrap();
	}
}
