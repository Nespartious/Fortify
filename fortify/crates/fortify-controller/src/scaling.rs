use serde::{Deserialize, Serialize};

/// Scaling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingDecision {
    ScaleUp,
    ScaleDown,
}

/// Scaling policy
pub struct ScalingPolicy {
    min_orchestrators: usize,
    max_orchestrators: usize,
    min_nodes: usize,
    max_nodes: usize,
    cpu_scale_up_threshold: f32,
    cpu_scale_down_threshold: f32,
    memory_scale_up_threshold: f32,
    memory_scale_down_threshold: f32,
}

impl ScalingPolicy {
    pub fn new(
        min_orchestrators: usize,
        max_orchestrators: usize,
        min_nodes: usize,
        max_nodes: usize,
    ) -> Self {
        Self {
            min_orchestrators,
            max_orchestrators,
            min_nodes,
            max_nodes,
            cpu_scale_up_threshold: 70.0,
            cpu_scale_down_threshold: 30.0,
            memory_scale_up_threshold: 70.0,
            memory_scale_down_threshold: 30.0,
        }
    }

    /// Determine if orchestrators should scale
    pub fn should_scale_orchestrators(
        &self,
        current_count: usize,
        cpu_usage: f32,
        memory_usage: f32,
    ) -> Option<ScalingDecision> {
        // Check if at limits
        if current_count >= self.max_orchestrators && self.should_scale_up(cpu_usage, memory_usage)
        {
            return None; // Can't scale up
        }

        if current_count <= self.min_orchestrators
            && self.should_scale_down(cpu_usage, memory_usage)
        {
            return None; // Can't scale down
        }

        // Determine scaling decision
        if self.should_scale_up(cpu_usage, memory_usage) && current_count < self.max_orchestrators {
            Some(ScalingDecision::ScaleUp)
        } else if self.should_scale_down(cpu_usage, memory_usage)
            && current_count > self.min_orchestrators
        {
            Some(ScalingDecision::ScaleDown)
        } else {
            None
        }
    }

    /// Determine if nodes should scale
    pub fn should_scale_nodes(
        &self,
        current_count: usize,
        cpu_usage: f32,
        memory_usage: f32,
    ) -> Option<ScalingDecision> {
        // Check if at limits
        if current_count >= self.max_nodes && self.should_scale_up(cpu_usage, memory_usage) {
            return None; // Can't scale up
        }

        if current_count <= self.min_nodes && self.should_scale_down(cpu_usage, memory_usage) {
            return None; // Can't scale down
        }

        // Determine scaling decision
        if self.should_scale_up(cpu_usage, memory_usage) && current_count < self.max_nodes {
            Some(ScalingDecision::ScaleUp)
        } else if self.should_scale_down(cpu_usage, memory_usage) && current_count > self.min_nodes
        {
            Some(ScalingDecision::ScaleDown)
        } else {
            None
        }
    }

    /// Check if should scale up based on resource usage
    /// Note: We DON'T scale up when memory is already high - that would make it worse!
    fn should_scale_up(&self, cpu_usage: f32, memory_usage: f32) -> bool {
        // Only scale up if CPU is high AND memory is still reasonable
        cpu_usage > self.cpu_scale_up_threshold && memory_usage < self.memory_scale_up_threshold
    }

    /// Check if should scale down based on resource usage
    fn should_scale_down(&self, cpu_usage: f32, memory_usage: f32) -> bool {
        cpu_usage < self.cpu_scale_down_threshold && memory_usage < self.memory_scale_down_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaling_policy_creation() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        assert_eq!(policy.min_orchestrators, 2);
        assert_eq!(policy.max_orchestrators, 10);
        assert_eq!(policy.min_nodes, 2);
        assert_eq!(policy.max_nodes, 20);
    }

    #[test]
    fn test_scale_up_orchestrators() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // High CPU should trigger scale up
        let decision = policy.should_scale_orchestrators(5, 80.0, 50.0);
        assert_eq!(decision, Some(ScalingDecision::ScaleUp));
    }

    #[test]
    fn test_scale_down_orchestrators() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // Low resource usage should trigger scale down
        let decision = policy.should_scale_orchestrators(5, 20.0, 20.0);
        assert_eq!(decision, Some(ScalingDecision::ScaleDown));
    }

    #[test]
    fn test_no_scale_at_max() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // At max, should not scale up even with high usage
        let decision = policy.should_scale_orchestrators(10, 80.0, 80.0);
        assert_eq!(decision, None);
    }

    #[test]
    fn test_no_scale_at_min() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // At min, should not scale down even with low usage
        let decision = policy.should_scale_orchestrators(2, 10.0, 10.0);
        assert_eq!(decision, None);
    }

    #[test]
    fn test_scale_up_nodes() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // High CPU with reasonable memory should trigger scale up
        // Note: We don't scale up when memory is high (would make it worse)
        let decision = policy.should_scale_nodes(10, 80.0, 50.0);
        assert_eq!(decision, Some(ScalingDecision::ScaleUp));
    }

    #[test]
    fn test_scale_down_nodes() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // Low usage should trigger scale down
        let decision = policy.should_scale_nodes(10, 25.0, 25.0);
        assert_eq!(decision, Some(ScalingDecision::ScaleDown));
    }

    #[test]
    fn test_no_scale_stable() {
        let policy = ScalingPolicy::new(2, 10, 2, 20);

        // Moderate usage should not trigger scaling
        let decision = policy.should_scale_orchestrators(5, 50.0, 50.0);
        assert_eq!(decision, None);
    }
}
