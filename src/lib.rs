#![forbid(unsafe_code)]

//! Advanced scheduling with ternary priorities.
//!
//! Provides job scheduling, resource pools, deadline tracking, priority inversion
//! detection, fair queuing, and ternary load balancing — all built around the
//! priority model where -1 = reject, 0 = queue normally, +1 = priority boost.

use std::collections::{HashMap, VecDeque};

/// A ternary priority value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Priority {
    /// Reject the job.
    Reject = -1,
    /// Normal queue priority.
    Normal = 0,
    /// Priority boost — schedule first.
    Priority = 1,
}

impl Priority {
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Priority::Reject),
            0 => Some(Priority::Normal),
            1 => Some(Priority::Priority),
            _ => None,
        }
    }
}

/// Unique identifier for a job.
pub type JobId = u64;

/// A schedulable job with ternary priority.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub priority: Priority,
    pub deadline: Option<u64>,
    pub resource_requirements: HashMap<String, u32>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

impl Job {
    pub fn new(id: JobId, priority: Priority, created_at: u64) -> Self {
        Self {
            id,
            priority,
            deadline: None,
            resource_requirements: HashMap::new(),
            created_at,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn require_resource(mut self, name: &str, amount: u32) -> Self {
        self.resource_requirements.insert(name.to_string(), amount);
        self
    }

    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn is_overdue(&self, current_time: u64) -> bool {
        self.deadline.map_or(false, |d| current_time > d && !self.is_complete())
    }

    pub fn wait_time(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.created_at)
    }
}

/// A resource pool with limited capacity.
#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub name: String,
    pub capacity: u32,
    pub allocated: u32,
}

impl ResourcePool {
    pub fn new(name: &str, capacity: u32) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            allocated: 0,
        }
    }

    pub fn available(&self) -> u32 {
        self.capacity.saturating_sub(self.allocated)
    }

    pub fn try_allocate(&mut self, amount: u32) -> bool {
        if amount <= self.available() {
            self.allocated += amount;
            true
        } else {
            false
        }
    }

    pub fn release(&mut self, amount: u32) {
        self.allocated = self.allocated.saturating_sub(amount);
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.allocated as f64 / self.capacity as f64
    }
}

/// Deadline tracker monitors jobs and identifies those at risk of missing deadlines.
#[derive(Debug, Clone)]
pub struct DeadlineTracker {
    pub current_time: u64,
}

impl DeadlineTracker {
    pub fn new() -> Self {
        Self { current_time: 0 }
    }

    pub fn advance(&mut self, ticks: u64) {
        self.current_time += ticks;
    }

    pub fn overdue_jobs(&self, jobs: &[Job]) -> Vec<JobId> {
        jobs.iter()
            .filter(|j| j.is_overdue(self.current_time))
            .map(|j| j.id)
            .collect()
    }

    pub fn at_risk_jobs(&self, jobs: &[Job], threshold: u64) -> Vec<JobId> {
        jobs.iter()
            .filter(|j| {
                if let Some(deadline) = j.deadline {
                    let remaining = deadline.saturating_sub(self.current_time);
                    !j.is_complete() && remaining <= threshold && remaining > 0
                } else {
                    false
                }
            })
            .map(|j| j.id)
            .collect()
    }
}

/// Priority inversion detection.
///
/// Priority inversion occurs when a high-priority job is waiting for
/// resources held by a lower-priority job. This detector scans active
/// jobs and their resource allocations to find such conflicts.
#[derive(Debug)]
pub struct PriorityInversionDetector;

impl PriorityInversionDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect priority inversions between running jobs and queued jobs.
    /// Returns pairs of (blocked_high_priority_job, blocking_low_priority_job).
    pub fn detect(
        &self,
        running: &[Job],
        queued: &[Job],
        pools: &[ResourcePool],
    ) -> Vec<(JobId, JobId)> {
        let mut inversions = Vec::new();

        // Find high-priority jobs in the queue
        for queued_job in queued {
            if queued_job.priority != Priority::Priority {
                continue;
            }

            // Check if queued job can't run because resources are taken
            for (resource, amount) in &queued_job.resource_requirements {
                for pool in pools {
                    if pool.name == *resource && pool.available() < *amount {
                        // Find which running job holds this resource
                        for running_job in running {
                            if running_job.priority == Priority::Normal
                                || running_job.priority == Priority::Reject
                            {
                                if let Some(alloc) = running_job.resource_requirements.get(resource) {
                                    if *alloc > 0 {
                                        inversions.push((queued_job.id, running_job.id));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        inversions
    }
}

/// Fair queue using round-robin scheduling within each priority level.
#[derive(Debug, Clone)]
pub struct FairQueue {
    pub priority_queue: VecDeque<Job>,
    pub normal_queue: VecDeque<Job>,
}

impl FairQueue {
    pub fn new() -> Self {
        Self {
            priority_queue: VecDeque::new(),
            normal_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, job: Job) -> bool {
        match job.priority {
            Priority::Reject => false,
            Priority::Normal => {
                self.normal_queue.push_back(job);
                true
            }
            Priority::Priority => {
                self.priority_queue.push_back(job);
                true
            }
        }
    }

    /// Dequeue the next job. Priority jobs go first, then round-robin normal.
    pub fn dequeue(&mut self) -> Option<Job> {
        if let Some(job) = self.priority_queue.pop_front() {
            return Some(job);
        }
        self.normal_queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.priority_queue.len() + self.normal_queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.priority_queue.is_empty() && self.normal_queue.is_empty()
    }
}

/// A job scheduler with ternary priority support and resource management.
#[derive(Debug)]
pub struct JobScheduler {
    pub queue: FairQueue,
    pub pools: HashMap<String, ResourcePool>,
    pub completed: Vec<Job>,
    pub rejected: Vec<JobId>,
    pub current_time: u64,
    pub next_job_id: JobId,
}

impl JobScheduler {
    pub fn new() -> Self {
        Self {
            queue: FairQueue::new(),
            pools: HashMap::new(),
            completed: Vec::new(),
            rejected: Vec::new(),
            current_time: 0,
            next_job_id: 1,
        }
    }

    pub fn add_pool(&mut self, name: &str, capacity: u32) {
        self.pools.insert(name.to_string(), ResourcePool::new(name, capacity));
    }

    pub fn submit(&mut self, priority: Priority) -> JobId {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let job = Job::new(id, priority, self.current_time);
        if !self.queue.enqueue(job) {
            self.rejected.push(id);
        }
        id
    }

    pub fn submit_job(&mut self, job: Job) -> JobId {
        let id = job.id;
        if !self.queue.enqueue(job) {
            self.rejected.push(id);
        }
        id
    }

    pub fn advance_time(&mut self, ticks: u64) {
        self.current_time += ticks;
    }

    /// Try to schedule the next job. Returns the job if resources are available.
    pub fn schedule_next(&mut self) -> Option<Job> {
        let job = self.queue.dequeue()?;
        
        // Check resource availability
        for (resource, amount) in &job.resource_requirements {
            if let Some(pool) = self.pools.get_mut(resource) {
                if !pool.try_allocate(*amount) {
                    // Can't allocate — put job back at front of queue
                    self.queue.enqueue(job);
                    return None;
                }
            }
        }

        let mut job = job;
        job.started_at = Some(self.current_time);
        Some(job)
    }

    /// Complete a job and release its resources.
    pub fn complete(&mut self, job: Job) {
        // Release resources
        for (resource, amount) in &job.resource_requirements {
            if let Some(pool) = self.pools.get_mut(resource) {
                pool.release(*amount);
            }
        }
        let mut job = job;
        job.completed_at = Some(self.current_time);
        self.completed.push(job);
    }

    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
}

/// Ternary load balancer distributes jobs across multiple workers.
///
/// Workers are rated as overloaded (-1), balanced (0), or underloaded (+1).
/// New jobs are sent to underloaded workers first, then balanced, avoiding overloaded.
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    pub workers: HashMap<u64, WorkerState>,
}

/// State of a worker node.
#[derive(Debug, Clone)]
pub struct WorkerState {
    pub id: u64,
    pub job_count: usize,
    pub capacity: usize,
}

impl WorkerState {
    pub fn new(id: u64, capacity: usize) -> Self {
        Self { id, job_count: 0, capacity }
    }

    /// Ternary load rating.
    pub fn load_rating(&self) -> Priority {
        let utilization = if self.capacity == 0 { 1.0 } else {
            self.job_count as f64 / self.capacity as f64
        };
        if utilization > 0.8 {
            Priority::Reject // overloaded
        } else if utilization > 0.5 {
            Priority::Normal // balanced
        } else {
            Priority::Priority // underloaded — prefer
        }
    }

    pub fn is_available(&self) -> bool {
        self.job_count < self.capacity
    }

    pub fn assign(&mut self) {
        self.job_count += 1;
    }

    pub fn release(&mut self) {
        self.job_count = self.job_count.saturating_sub(1);
    }
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self { workers: HashMap::new() }
    }

    pub fn add_worker(&mut self, worker: WorkerState) {
        self.workers.insert(worker.id, worker);
    }

    /// Select the best worker for a new job using ternary load balancing.
    /// Prefers underloaded, then balanced, skips overloaded.
    /// Returns None if all workers are overloaded or full.
    pub fn select_worker(&self) -> Option<u64> {
        // First try underloaded
        let mut best_balanced: Option<&WorkerState> = None;
        for worker in self.workers.values() {
            if !worker.is_available() { continue; }
            match worker.load_rating() {
                Priority::Priority => return Some(worker.id), // underloaded
                Priority::Normal => {
                    if best_balanced.is_none()
                        || worker.job_count < best_balanced.unwrap().job_count
                    {
                        best_balanced = Some(worker);
                    }
                }
                Priority::Reject => {} // overloaded, skip
            }
        }
        best_balanced.map(|w| w.id)
    }

    /// Assign a job to the best available worker.
    pub fn assign_job(&mut self) -> Option<u64> {
        let worker_id = self.select_worker()?;
        self.workers.get_mut(&worker_id).unwrap().assign();
        Some(worker_id)
    }

    /// Release a job from a worker.
    pub fn release_job(&mut self, worker_id: u64) {
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.release();
        }
    }

    /// Count workers by load rating.
    pub fn worker_distribution(&self) -> (usize, usize, usize) {
        let mut overloaded = 0;
        let mut balanced = 0;
        let mut underloaded = 0;
        for w in self.workers.values() {
            match w.load_rating() {
                Priority::Reject => overloaded += 1,
                Priority::Normal => balanced += 1,
                Priority::Priority => underloaded += 1,
            }
        }
        (overloaded, balanced, underloaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_i8() {
        assert_eq!(Priority::from_i8(-1), Some(Priority::Reject));
        assert_eq!(Priority::from_i8(0), Some(Priority::Normal));
        assert_eq!(Priority::from_i8(1), Some(Priority::Priority));
        assert_eq!(Priority::from_i8(2), None);
    }

    #[test]
    fn test_job_new() {
        let job = Job::new(1, Priority::Normal, 0);
        assert_eq!(job.id, 1);
        assert_eq!(job.priority, Priority::Normal);
        assert!(job.deadline.is_none());
        assert!(!job.is_complete());
    }

    #[test]
    fn test_job_deadline_overdue() {
        let job = Job::new(1, Priority::Normal, 0).with_deadline(10);
        assert!(!job.is_overdue(5));
        assert!(!job.is_overdue(10));
        assert!(job.is_overdue(11));
    }

    #[test]
    fn test_job_wait_time() {
        let job = Job::new(1, Priority::Normal, 5);
        assert_eq!(job.wait_time(10), 5);
        assert_eq!(job.wait_time(5), 0);
    }

    #[test]
    fn test_resource_pool_allocate_release() {
        let mut pool = ResourcePool::new("cpu", 10);
        assert_eq!(pool.available(), 10);
        assert!(pool.try_allocate(5));
        assert_eq!(pool.available(), 5);
        assert!(!pool.try_allocate(6));
        pool.release(3);
        assert_eq!(pool.available(), 8);
    }

    #[test]
    fn test_resource_pool_utilization() {
        let mut pool = ResourcePool::new("mem", 100);
        pool.try_allocate(50);
        assert!((pool.utilization() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_deadline_tracker_overdue() {
        let mut tracker = DeadlineTracker::new();
        let jobs = vec![
            Job::new(1, Priority::Normal, 0).with_deadline(5),
            Job::new(2, Priority::Normal, 0).with_deadline(20),
        ];
        tracker.advance(10);
        let overdue = tracker.overdue_jobs(&jobs);
        assert_eq!(overdue, vec![1]);
    }

    #[test]
    fn test_deadline_tracker_at_risk() {
        let mut tracker = DeadlineTracker::new();
        let jobs = vec![
            Job::new(1, Priority::Normal, 0).with_deadline(15),
            Job::new(2, Priority::Normal, 0).with_deadline(30),
        ];
        tracker.advance(10);
        let at_risk = tracker.at_risk_jobs(&jobs, 10);
        assert!(at_risk.contains(&1));
        assert!(!at_risk.contains(&2));
    }

    #[test]
    fn test_fair_queue_reject() {
        let mut q = FairQueue::new();
        let job = Job::new(1, Priority::Reject, 0);
        assert!(!q.enqueue(job));
    }

    #[test]
    fn test_fair_queue_priority_first() {
        let mut q = FairQueue::new();
        q.enqueue(Job::new(1, Priority::Normal, 0));
        q.enqueue(Job::new(2, Priority::Priority, 0));
        let next = q.dequeue();
        assert_eq!(next.unwrap().id, 2); // Priority goes first
    }

    #[test]
    fn test_fair_queue_fifo_within_priority() {
        let mut q = FairQueue::new();
        q.enqueue(Job::new(1, Priority::Normal, 0));
        q.enqueue(Job::new(2, Priority::Normal, 0));
        let next = q.dequeue();
        assert_eq!(next.unwrap().id, 1); // FIFO
    }

    #[test]
    fn test_job_scheduler_submit() {
        let mut scheduler = JobScheduler::new();
        let id1 = scheduler.submit(Priority::Normal);
        let id2 = scheduler.submit(Priority::Priority);
        let id3 = scheduler.submit(Priority::Reject);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert!(scheduler.rejected.contains(&id3));
    }

    #[test]
    fn test_job_scheduler_schedule() {
        let mut scheduler = JobScheduler::new();
        scheduler.submit(Priority::Normal);
        scheduler.submit(Priority::Priority);
        let job = scheduler.schedule_next();
        assert_eq!(job.unwrap().priority, Priority::Priority);
    }

    #[test]
    fn test_job_scheduler_complete() {
        let mut scheduler = JobScheduler::new();
        scheduler.add_pool("cpu", 4);
        let mut job = Job::new(1, Priority::Normal, 0).require_resource("cpu", 2);
        scheduler.queue.enqueue(job);
        let mut scheduled = scheduler.schedule_next().unwrap();
        scheduler.complete(scheduled);
        assert_eq!(scheduler.completed_count(), 1);
    }

    #[test]
    fn test_priority_inversion_detection() {
        let detector = PriorityInversionDetector::new();
        let running = vec![Job::new(1, Priority::Normal, 0).require_resource("cpu", 4)];
        let queued = vec![Job::new(2, Priority::Priority, 0).require_resource("cpu", 4)];
        let pools = vec![ResourcePool::new("cpu", 4)]; // fully allocated
        // We need to simulate that the pool is fully allocated
        let mut pools = pools;
        pools[0].try_allocate(4); // all taken
        let inversions = detector.detect(&running, &queued, &pools);
        assert_eq!(inversions.len(), 1);
        assert_eq!(inversions[0], (2, 1));
    }

    #[test]
    fn test_load_balancer_select() {
        let mut lb = LoadBalancer::new();
        lb.add_worker(WorkerState::new(1, 10)); // 0/10 = underloaded
        lb.add_worker(WorkerState::new(2, 10)); // 0/10 = underloaded
        let selected = lb.select_worker();
        assert!(selected.is_some());
    }

    #[test]
    fn test_load_balancer_prefer_underloaded() {
        let mut lb = LoadBalancer::new();
        let mut w1 = WorkerState::new(1, 10);
        w1.job_count = 9; // overloaded
        let mut w2 = WorkerState::new(2, 10);
        w2.job_count = 2; // underloaded
        lb.add_worker(w1);
        lb.add_worker(w2);
        let selected = lb.select_worker();
        assert_eq!(selected, Some(2));
    }

    #[test]
    fn test_load_balancer_assign_release() {
        let mut lb = LoadBalancer::new();
        lb.add_worker(WorkerState::new(1, 2));
        let worker = lb.assign_job();
        assert_eq!(worker, Some(1));
        assert_eq!(lb.workers.get(&1).unwrap().job_count, 1);
        lb.release_job(1);
        assert_eq!(lb.workers.get(&1).unwrap().job_count, 0);
    }

    #[test]
    fn test_load_balancer_distribution() {
        let mut lb = LoadBalancer::new();
        let mut w1 = WorkerState::new(1, 10);
        w1.job_count = 9; // overloaded
        let w2 = WorkerState::new(2, 10); // underloaded
        let mut w3 = WorkerState::new(3, 10);
        w3.job_count = 6; // balanced
        lb.add_worker(w1);
        lb.add_worker(w2);
        lb.add_worker(w3);
        let (over, bal, under) = lb.worker_distribution();
        assert_eq!(over, 1);
        assert_eq!(bal, 1);
        assert_eq!(under, 1);
    }

    #[test]
    fn test_worker_load_rating() {
        let mut w = WorkerState::new(1, 10);
        w.job_count = 2;
        assert_eq!(w.load_rating(), Priority::Priority); // 20% = underloaded
        w.job_count = 6;
        assert_eq!(w.load_rating(), Priority::Normal); // 60% = balanced
        w.job_count = 9;
        assert_eq!(w.load_rating(), Priority::Reject); // 90% = overloaded
    }

    #[test]
    fn test_job_resource_requirements() {
        let job = Job::new(1, Priority::Normal, 0)
            .require_resource("cpu", 2)
            .require_resource("mem", 4);
        assert_eq!(job.resource_requirements.get("cpu"), Some(&2));
        assert_eq!(job.resource_requirements.get("mem"), Some(&4));
    }
}
