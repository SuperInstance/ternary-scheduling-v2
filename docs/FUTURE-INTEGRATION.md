# Future Integration: ternary-scheduling-v2

## Current State
Advanced job scheduling with ternary priorities: `Priority` (Reject/Normal/Priority = -1/0/+1), `Job` with deadlines and resource requirements, `ResourcePool`, `DeadlineTracker`, priority inversion detection, fair queuing, and ternary load balancing.

## Integration Opportunities

### With Fleet Resource Coordination
The entire fleet is a `ResourcePool`. Each room is a resource unit with specific capabilities (ESP32 = low compute, DGX = high compute). Jobs (agent tasks, cell tick cycles, inference runs) are scheduled across the fleet. `Priority::Reject` means the fleet can't handle it; `Priority::Normal` queues it; `Priority::Priority` preempts. `DeadlineTracker` ensures time-critical room operations complete before their tick deadline.

### With ternary-room (Room Scheduling)
Room provisioning is a scheduling problem. `Job::resource_requirements` specifies what a room needs (CPU, memory, GPU). `ResourcePool` tracks which rooms are available. When a room needs to spawn a child room (cell division), it creates a `Job` with the child's requirements, and the scheduler finds the best physical node to host it.

### With construct-core Tiers
`Priority` maps to compute tiers: Reject = below Basic tier, Normal = Standard tier, Priority = Advanced/Expert tier. `ResourcePool` has one pool per tier. Jobs are first assigned to their minimum tier's pool, then promoted if resources are available in higher tiers.

## Potential in Mature Systems
The scheduler becomes the fleet's operating system. Every computation is a `Job`. Every resource — CPU cycle, memory byte, network packet — is tracked in a `ResourcePool`. Priority inversion detection prevents high-priority room operations from being blocked by low-priority ones. Fair queuing ensures no room or agent is starved.

## Cross-Pollination Ideas
- `Job::resource_requirements` could use `ternary-registry::Skill` to describe what capabilities a job needs
- Fair queuing could use `conservation-matrix-rs` ratios to ensure the 294:1 avoid:choose ratio is maintained in scheduling decisions
- `DeadlineTracker` connects to `ternary-cell`'s tick cycle — each cell tick has a scheduling deadline
- Load balancing could use `ternary-tensor` for representing multi-dimensional resource state across the fleet

## Dependencies for Next Steps
- Integration with ternary-distributed for distributed scheduling (not single-node)
- Real fleet resource metrics for ResourcePool population
- Priority inversion resolution strategy (priority inheritance vs. priority ceiling)
- Connection to ternary-protocol for cross-node job submission
