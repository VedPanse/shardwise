# Shardwise

Shardwise is a planned fault-tolerant distributed execution framework for AI and data-processing workloads. It will coordinate dependency-aware tasks through a Paxos-inspired metadata service, execute them across replicated worker groups, and recover from node failures.

The project is designed to run locally on Apple Silicon using free, open-source infrastructure. No cloud account, paid service, or GPU is required.

## Current status

The repository currently contains an initial Rust scaffold. The services, protocols, workloads, and guarantees described below are the implementation targets, not completed capabilities. The starter executable is not yet a working framework.

## Goals

- Implement durable leader election and metadata consensus with strict-majority recovery.
- Separate replicated coordination metadata from replicated worker storage.
- Schedule task dependency graphs and recover interrupted work without publishing duplicate results.
- Demonstrate real AI and data-processing workloads, rather than only synthetic tasks.
- Measure throughput, latency, resource use, and recovery behavior under controlled failures.
- Keep development, demonstrations, and benchmarks entirely local and free.

## Architecture

```mermaid
flowchart TD
    Client[Client / CLI] --> API[Rust API]
    API --> Meta[Replicated metadata service\nPaxos-inspired consensus]
    Meta --> Scheduler[Rust DAG scheduler]
    Scheduler --> Kafka[Kafka\nTask dispatch and events]
    Kafka --> Workers[Worker groups\nRust agents + Python task processes]
    Workers --> Storage[Replicated local storage\nInputs, checkpoints, outputs]
    Workers --> Meta
    API --> Redis[Redis\nDisposable cache]
    Kafka --> History[History projector]
    History --> Postgres[PostgreSQL\nQueryable job history]
    Monitor[Prometheus + Grafana] -.-> Meta
    Monitor -.-> Scheduler
    Monitor -.-> Workers
```

### Metadata and consensus

A separate metadata quorum will own authoritative coordination state: job graphs, task transitions, worker membership, replica placement, and accepted results.

The Paxos-inspired protocol will cover election, quorum-backed log replication, durable promises and accepted values, and replay after restart. A typical local configuration will use three metadata nodes, requiring two votes to make progress. A five-node configuration would require three votes.

Leader changes must preserve previously chosen values. Terms or ballots and fencing checks will prevent an obsolete leader or task attempt from committing new authoritative state. Nodes without a majority will stop metadata writes until quorum is restored; a minority must never elect itself into authority.

This targets crash and network-partition failures, not malicious nodes. Protocol safety and recovery will need deterministic tests and fault injection before these guarantees can be claimed as implemented.

### Hierarchical replicated storage

The metadata service will track data ownership and placement without storing bulk workload data in its consensus log. Worker groups will replicate task inputs, available checkpoints, and outputs across local disks belonging to distinct worker nodes.

Replica placement, acknowledgements, checksums, and repair will be explicit. A task result will become authoritative only after its configured storage durability requirement is satisfied and the metadata quorum accepts its commit. Metadata quorum size and data replication factor are separate settings.

After a worker fails, another worker can read surviving replicas and resume from a supported checkpoint or retry the task. Lost replicas will be rebuilt from surviving copies. If every copy of required data is lost, recovery requires a reproducible source or resubmission.

### Dependency-aware execution

Clients will submit directed acyclic graphs (DAGs) whose tasks declare dependencies, inputs, resource needs, and retry policies. The scheduler will reject cycles and release a task only after its prerequisites have committed successfully.

Rust worker agents will handle dispatch, storage, heartbeats, and task lifecycle management. Python processes will execute registered AI and data-processing task functions. The initial interface will support user-defined graphs built from registered task types; arbitrary untrusted code execution is outside the initial scope.

The first example workloads will be:

- **AI:** partitioned gradient computation for a small CPU model, followed by gradient aggregation and comparison with a single-process reference.
- **Data processing:** partitioned transforms and aggregations, with intermediate artifacts shared through replicated storage.

### Infrastructure responsibilities

| Component | Responsibility |
| --- | --- |
| Rust metadata service | Authoritative coordination state, leader election, consensus, durable recovery |
| Rust API and scheduler | Job submission, graph validation, dependency resolution, task placement |
| Rust worker agents | Task lifecycle, storage replication, health reporting, recovery |
| Python task processes | AI and data-processing implementations using native numerical libraries where useful |
| Kafka | Durable task dispatch and lifecycle events; delivery may be repeated |
| Redis | Disposable caching; cache loss must not invalidate authoritative state |
| PostgreSQL | Queryable job history and reporting projections; coordination authority remains in the metadata service |
| Prometheus and Grafana | Cluster health, workload metrics, and recovery visibility |
| Docker Compose | Reproducible local orchestration |

Kafka publication and metadata commits will require a recoverable publication workflow and reconciliation. Consumers will deduplicate repeated events. PostgreSQL projections may lag authoritative state, and APIs will distinguish historical views from authoritative status.

Rust is the default for performance-sensitive infrastructure. Python provides access to the AI and data ecosystem. Go can be introduced for a bounded component if measurements or a concrete library advantage justify another runtime; adding languages is not itself a performance optimization.

## Failure and execution semantics

| Condition | Intended behavior |
| --- | --- |
| Metadata leader crashes | A surviving majority elects a leader and recovers chosen metadata |
| Metadata network partitions | Only a partition with a strict majority can advance authoritative state |
| Worker crashes or stops reporting | Work is reassigned after failure detection; stale attempts are fenced at commit |
| Task or event is delivered twice | Repeated execution is possible; authoritative result commits are deduplicated |
| Data replica fails | Read surviving copies and repair replication when capacity is available |
| Redis is unavailable | Bypass the cache and use durable sources |
| PostgreSQL is unavailable | History views may be unavailable or stale; projection resumes after recovery |
| Kafka is unavailable | Dispatch and event processing pause; durable coordination state supports reconciliation after recovery |

Execution will be **at least once**, with a single accepted result per logical task. This does not guarantee exactly-once execution or exactly-once effects in external systems. Tasks that write external state must provide their own idempotency mechanism.

An acknowledged result's survival depends on the configured replica durability policy and surviving disks. Running several containers on one laptop demonstrates process and simulated network failures; it does not protect against losing that laptop or its disk.

## Local development and deployment

All planned services will run on one Apple Silicon Mac through Docker Compose, using ARM64-compatible images and CPU workloads. Replica counts, worker concurrency, workload size, and observability services will be configurable to fit available memory.

No setup command is published yet because the runnable cluster has not been implemented. A working local quickstart will be added with the first end-to-end milestone.

## Implementation milestones

1. **Protocol foundation:** define state machines and failure assumptions; implement durable consensus, leader election, replay, and deterministic protocol tests.
2. **Replicated storage:** implement worker identities, artifact checksums, configurable replication acknowledgements, replica reads, and repair.
3. **End-to-end execution:** add the submission API, DAG scheduler, Kafka dispatch, Python task execution, and fenced result commits.
4. **Recovery and observability:** add failure detection, retries, reconciliation, Redis caching, PostgreSQL history, and cluster dashboards.
5. **Workload demonstrations:** run the gradient and data-processing pipelines and validate their outputs against reference implementations.
6. **Reliability and performance evidence:** automate failure scenarios, benchmark the local cluster, and document results and limits.

## Validation and performance

Correctness comes before optimization. Tests will exercise concurrent proposals, leader changes, partitions, restarts, delayed messages, duplicate deliveries, and stale workers. Storage checks will cover corrupted or missing replicas and recovery from surviving copies.

Benchmarks will report task throughput, end-to-end and scheduling latency, metadata commit latency, recovery time, replication overhead, and CPU and memory use. Every result will include hardware, workload, payload sizes, concurrency, replica counts, and comparison with a single-worker baseline.

The performance goal is efficient execution with demonstrated improvements, not an unsupported claim to be the fastest framework. Local experiments can establish correctness and scaling behavior within one machine; claims about large-scale, multi-machine operation require corresponding evidence.

## Project intent

Shardwise is an educational systems project intended to demonstrate distributed coordination, replicated storage, dependency-aware execution, and measurable fault recovery. Resume claims will be backed by implemented behavior, reproducible tests, and published measurements as the milestones are completed.

## Author

**Ved Panse** — [vedpanse.com](https://vedpanse.com)

UC San Diego | B.S. Math-CS & Data Science
