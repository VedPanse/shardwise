# Shardwise TODO

This checklist tracks implementation of the [README baselines](README.md#implementation-baselines). All items start unchecked because the repository currently contains only a Rust scaffold. Finish and demonstrate each baseline before adding the next one's infrastructure.

## Baseline 1 — Minimal end-to-end execution

### 1. Project and contracts

- [ ] Replace the starter with coordinator and worker entry points in the existing Rust project.
- [ ] Define shared job IDs, attempt IDs, task input/output types, and structured errors.
- [ ] Define legal transitions: `queued → running → succeeded | failed`; terminal state is immutable.
- [ ] Define the `sum_numbers` task schema, numeric validation, and request size limit.
- [ ] Add a small Python task registry and a JSON stdin/stdout contract; send task diagnostics to stderr.
- [ ] Keep configuration minimal: bind address, coordinator URL, Python executable, polling interval, and task timeout.

### 2. Coordinator

- [ ] Implement `GET /health`.
- [ ] Implement `POST /jobs` with validation, unique IDs, and `202` acceptance.
- [ ] Store jobs and the pending queue in memory behind synchronized access.
- [ ] Implement `GET /jobs/{id}` with status, result/error, and `404` for unknown IDs.
- [ ] Implement an internal HTTP claim endpoint that atomically assigns a queued task and attempt ID; return an explicit no-work response when empty.
- [ ] Implement an internal result endpoint that validates the assigned attempt and accepts one terminal outcome.
- [ ] Make identical repeated completion reports idempotent; reject conflicting and unassigned reports.

### 3. Worker and Python execution

- [ ] Run the worker as a separate process and poll the coordinator without busy looping.
- [ ] Execute one claimed task at a time using the configured Python executable.
- [ ] Implement `sum_numbers` without third-party Python dependencies.
- [ ] Parse and validate runner output; capture nonzero exit codes, invalid output, and useful error details.
- [ ] Enforce an execution timeout, terminate/reap the task process, and report failure.
- [ ] Retry completion reports after transient connection failures using the same attempt ID.
- [ ] Emit structured logs containing job ID, attempt ID, and state changes.
- [ ] Bind local services to loopback by default; document that remote/untrusted execution is not supported yet.

### 4. Verify and document

- [ ] Test invalid input, unknown task types, oversized requests, and unknown job IDs.
- [ ] Test atomic claims and immutable terminal states, including duplicate and stale completion reports.
- [ ] Add a smoke test that starts coordinator and worker, submits `[1, 2, 3, 4]`, polls with a deadline, and asserts a successful result of `10`.
- [ ] Use test-only runner fixtures to verify Python failure, malformed output, and timeout become failed jobs.
- [ ] Ensure the smoke test cleans up its child processes and reports actionable failures.
- [ ] Add exact native install/start/submit/status commands to the README after they work.
- [ ] Document that coordinator restart loses all jobs and worker death has no automatic recovery yet.
- [ ] Run Rust formatting, linting, relevant tests, and the end-to-end smoke test.

**Exit gate:** a new local checkout can run the documented successful and failed task demonstrations with only Rust and Python. No DAG engine, consensus, databases, broker, or dashboard is needed to finish this baseline.

## Baseline 2 — DAG scheduling

- [ ] Introduce job graphs, per-task state, and dependency output references.
- [ ] Reject cycles, duplicate task IDs, and missing dependencies before accepting a graph.
- [ ] Release tasks only after all prerequisites succeed; define downstream failure propagation.
- [ ] Support two or more workers with bounded in-flight work.
- [ ] Build a partitioned transform/aggregation example and its single-process reference.
- [ ] Verify dependency ordering, concurrent independent tasks, and terminal failure propagation.
- [ ] Document and demonstrate the multi-worker DAG flow.

**Exit gate:** the data-processing graph matches its reference and exercises real dependency scheduling across two workers.

## Baseline 3 — Consensus and durable coordination

- [ ] Specify protocol messages, safety invariants, fixed membership, quorum rules, and failure assumptions before implementation.
- [ ] Implement persistent promises, accepted values, ballots, and crash-safe log replay.
- [ ] Implement leader election and recovery that preserves previously chosen values.
- [ ] Implement quorum-backed metadata writes and a read strategy that cannot return stale authoritative state as current.
- [ ] Route job acceptance, scheduling assignments, and result transitions through replicated metadata.
- [ ] Rebuild scheduler state from committed metadata after restart.
- [ ] Add deterministic tests for competing proposals, message duplication/reordering, partitions, crashes, and restart.
- [ ] Demonstrate a three-node cluster: two nodes can progress, one cannot; previously acknowledged state survives a leader crash.

**Exit gate:** demonstrated majority recovery and tested consensus safety, integrated with the job flow.

## Baseline 4 — Storage replication and recovery

- [ ] Add disk-backed artifacts with IDs, checksums, and explicit durable-write acknowledgements.
- [ ] Store replica placement in metadata; keep artifact bytes outside the consensus log.
- [ ] Replicate inputs and outputs to two distinct workers in the initial demo.
- [ ] Gate result commits on the configured number of durable data replicas.
- [ ] Add heartbeat-based failure detection and bounded task retries with new attempt IDs.
- [ ] Reject stale attempt commits after reassignment; keep one accepted logical result.
- [ ] Retrieve surviving replicas, detect corruption, and repair missing copies.
- [ ] Test a worker crash during execution and after result acknowledgement, plus duplicate and delayed reports.
- [ ] Document behavior when all required replicas are lost and the limits of single-host deployment.

**Exit gate:** a worker failure preserves acknowledged data and allows interrupted work to complete from surviving inputs.

## Baseline 5 — Kafka and operational tooling

- [ ] Define versioned dispatch/event schemas, partitioning, consumer acknowledgement, and replay behavior.
- [ ] Implement a consensus-backed publication intent/outbox and replay-safe Kafka publication.
- [ ] Replace worker polling with Kafka consumption; validate assignments against authoritative metadata.
- [ ] Reconcile committed state with outstanding dispatches and deduplicate repeated events.
- [ ] Add Redis as a bypassable cache with an explicit invalidation policy.
- [ ] Add PostgreSQL job-history projections with idempotent replay and visible projection lag.
- [ ] Add metrics and dashboards for quorum health, queues, task outcomes, worker health, and recovery.
- [ ] Provide ARM64 Compose services, persistent volumes, resource settings, and optional observability profiles.
- [ ] Test Kafka interruption, Redis loss, and PostgreSQL outage/recovery.
- [ ] Document local startup, shutdown, recovery, and deliberate data reset separately.

**Exit gate:** infrastructure outages recover as documented without conflicting authoritative results or lost committed task transitions.

## Baseline 6 — Workloads and evidence

- [ ] Implement a small CPU gradient-computation DAG with a numerical reference and explicit tolerances.
- [ ] Validate both AI and data-processing workloads end to end.
- [ ] Automate leader crash, quorum loss, worker crash, replica loss, and infrastructure outage demonstrations.
- [ ] Measure throughput, scheduling/end-to-end latency, metadata commit latency, recovery time, and resource use.
- [ ] Record hardware, payload sizes, concurrency, replica counts, and a single-worker comparison.
- [ ] Publish reproducible local results and distinguish implemented guarantees from remaining limitations.

**Exit gate:** correctness and reliability claims have reproducible evidence; performance claims match the scale actually measured.

## Later extensions — Outside the initial baselines

- [ ] Task-specific checkpoint/resume support.
- [ ] Resource-aware placement beyond basic concurrency limits.
- [ ] Dynamic metadata membership with a safe reconfiguration protocol.
- [ ] Consider a Go component only for a concrete measured or ecosystem advantage.
- [ ] Multi-machine benchmarks only if free hardware becomes available; no cloud spend is required.
