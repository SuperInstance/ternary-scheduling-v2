# ternary-scheduling-v2: Advanced scheduling with {-1, 0, +1} priorities

A Rust scheduling library where job priority is ternary: **-1 rejects the job**, **0 queues it normally**, and **+1 gives it priority boost**. Includes resource pools, deadline tracking, priority inversion detection, fair queuing, and load balancing.

## Why This Exists

Most schedulers use numeric priorities (0-255, or nice values). That's overkill for many systems and makes the "reject immediately" case awkward (you need a special flag or sentinel value). Ternary priority is the simplest model that captures the three decisions a scheduler actually makes: refuse, accept normally, or expedite. This library builds a full scheduling framework around that three-valued model.

## Core Concepts

**Ternary priority** — Three scheduling decisions: `Reject` (-1, discard the job immediately), `Normal` (0, queue it), `Priority` (+1, queue it ahead of normal jobs).

**Resource pool** — A named pool with fixed capacity (e.g., "cpu" with 4 units). Jobs declare requirements; the scheduler only starts a job when all its resources are available.

**Deadline tracking** — Jobs can have deadlines (a timestamp by which they must complete). The tracker identifies overdue jobs and jobs at risk of missing their deadline.

**Priority inversion** — A scheduling failure where a high-priority job is blocked waiting for resources held by a lower-priority job. Classic real-time systems problem.

**Fair queue** — Round-robin within each priority level. Priority jobs are dequeued first; normal jobs are FIFO within their level.

**Load balancing** — Distributing jobs across multiple worker nodes. Workers are classified as overloaded, balanced, or underloaded based on utilization thresholds.

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-scheduling-v2 = "0.1"
```

```rust
use ternary_scheduling_v2::*;

let mut scheduler = JobScheduler::new();
scheduler.add_pool("cpu", 4);
scheduler.add_pool("gpu", 2);

// Submit jobs with different priorities
let normal = scheduler.submit(Priority::Normal);
let urgent = scheduler.submit(Priority::Priority);
let _rejected = scheduler.submit(Priority::Reject); // immediately rejected

// Schedule the next job
if let Some(job) = scheduler.schedule_next() {
    println!("Running job {} with priority {:?}", job.id, job.priority);
    scheduler.complete(job);
}
```

## API Overview

| Type | Description |
|------|-------------|
| `Priority` | Ternary priority: `Reject`, `Normal`, `Priority` |
| `Job` | A schedulable unit with priority, deadline, and resource requirements |
| `ResourcePool` | A named capacity pool that tracks allocation |
| `DeadlineTracker` | Monitors jobs for overdue and at-risk deadlines |
| `PriorityInversionDetector` | Finds high-priority jobs blocked by low-priority jobs |
| `FairQueue` | Two-level priority queue (priority FIFO + normal FIFO) |
| `JobScheduler` | Full scheduler with queues, resource pools, and completion tracking |
| `LoadBalancer` | Distributes jobs across workers using ternary load ratings |
| `WorkerState` | A worker node's current job count and capacity |

## How It Works

**JobScheduler** maintains a `FairQueue` for pending jobs and a set of named `ResourcePool`s. On `submit`, jobs with `Reject` priority are immediately rejected and logged. `Normal` and `Priority` jobs are enqueued. On `schedule_next`, the highest-priority job is dequeued; if its resource requirements can be satisfied, resources are allocated and the job is returned. If not, the job goes back to the front of the queue.

**FairQueue** is two `VecDeque`s: one for priority jobs, one for normal. Dequeue always pops from the priority queue first. This means starvation is possible for normal jobs under sustained priority load (see Limitations).

**LoadBalancer** rates each worker's utilization against two thresholds: above 80% = overloaded (skip), 50-80% = balanced (accept as fallback), below 50% = underloaded (preferred). The `select_worker` method picks the least-loaded underloaded worker, then falls back to the least-loaded balanced worker.

**PriorityInversionDetector** does a pairwise comparison: for each queued priority job, it checks if any running normal/low-priority job holds resources that the priority job needs but can't get due to pool exhaustion.

## Known Limitations

- **No starvation prevention for normal jobs**: Under sustained priority load, normal jobs may wait indefinitely. A production scheduler would need aging (gradually boosting normal job priority over time).
- **No preemption**: Once a job is scheduled and running, it can't be preempted for a higher-priority job. Resources are only released on explicit `complete()`.
- **Single-threaded**: The scheduler is not thread-safe. For multi-threaded use, wrap in a `Mutex` or use message passing.
- **No persistence**: All state is in-memory. On crash, all queue state is lost.

## Use Cases

- **Task queue in a ternary web service** — API requests classified as reject (rate-limited), normal, or priority (premium users).
- **Robotics job scheduler** — Control tasks with reject (unsafe), normal (routine), priority (safety-critical) priorities, with hardware resource pools.
- **Build system** — Compilation jobs where some are normal and some are on the critical path (priority), with CPU/memory resource pools.
- **Multi-node job distribution** — Load balancer spreading jobs across worker machines, avoiding overloaded nodes.

## Ecosystem Context

Part of the SuperInstance ternary computing ecosystem. Related crates:

- `ternary-scheduling` — Simpler scheduler without resource pools or load balancing
- `ternary-locks` — Synchronization primitives for ternary concurrent systems
- `ternary-protocol` — Wire protocol for distributing ternary scheduling messages

This crate extends the basic scheduling model with resource management, deadline awareness, and multi-node support.

## License

MIT

## See Also
- **ternary-scheduling** — related
- **ternary-sync** — related
- **ternary-room** — related
- **ternary-consensus** — related
- **ternary-platoon** — related

