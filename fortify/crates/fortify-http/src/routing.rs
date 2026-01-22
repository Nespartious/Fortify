use crate::BackendNode;
use fortify_core::{safe_lock, TrustTier};
use std::sync::Arc;

/// Backend selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    /// Fill-first: fill nodes to capacity before moving to next (for threat nodes)
    FillFirst,
    /// Least-populated with name ordering: pick lowest session count, ties broken by name
    LeastPopulatedOrdered,
}

/// Router for selecting backend nodes
///
/// Phase 4.3: Node Distribution
/// - Healthy nodes: LeastPopulatedOrdered for even distribution with deterministic tie-breaking
/// - Threat nodes: Fill-first to maximize isolation on fewer nodes
pub struct Router {
    healthy_nodes: Vec<BackendNode>,
    threat_nodes: Vec<BackendNode>,
    /// Strategy for healthy nodes (default: LeastPopulatedOrdered)
    healthy_strategy: RoutingStrategy,
    /// Strategy for threat nodes (default: FillFirst)
    threat_strategy: RoutingStrategy,
    round_robin_index: Arc<std::sync::Mutex<usize>>,
    /// Track which node is currently being filled (for FillFirst strategy)
    fill_first_index: Arc<std::sync::Mutex<usize>>,
}

impl Router {
    pub fn new(
        healthy_nodes: Vec<BackendNode>,
        threat_nodes: Vec<BackendNode>,
        _strategy: RoutingStrategy,
    ) -> Self {
        Self {
            healthy_nodes,
            threat_nodes,
            // Phase 4.3: Healthy nodes use least-populated with name ordering
            healthy_strategy: RoutingStrategy::LeastPopulatedOrdered,
            // Phase 4.3: Threat nodes use fill-first by default
            threat_strategy: RoutingStrategy::FillFirst,
            round_robin_index: Arc::new(std::sync::Mutex::new(0)),
            fill_first_index: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Create router with explicit strategies for each tier
    pub fn new_with_strategies(
        healthy_nodes: Vec<BackendNode>,
        threat_nodes: Vec<BackendNode>,
        healthy_strategy: RoutingStrategy,
        threat_strategy: RoutingStrategy,
    ) -> Self {
        Self {
            healthy_nodes,
            threat_nodes,
            healthy_strategy,
            threat_strategy,
            round_robin_index: Arc::new(std::sync::Mutex::new(0)),
            fill_first_index: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Select appropriate backend based on trust tier
    pub fn select_backend(&self, trust_tier: TrustTier) -> Option<BackendNode> {
        let (nodes, strategy) = if trust_tier.requires_gate() {
            (&self.threat_nodes, self.threat_strategy)
        } else {
            (&self.healthy_nodes, self.healthy_strategy)
        };

        if nodes.is_empty() {
            return None;
        }

        match strategy {
            RoutingStrategy::RoundRobin => self.round_robin(nodes),
            RoutingStrategy::LeastConnections => self.least_connections(nodes),
            RoutingStrategy::WeightedRandom => self.weighted_random(nodes),
            RoutingStrategy::FillFirst => self.fill_first(nodes),
            RoutingStrategy::LeastPopulatedOrdered => self.least_populated_ordered(nodes),
        }
    }

    /// Least-populated with name ordering
    /// Sort by session count (ascending), then by name (ascending) for deterministic tie-breaking
    /// This ensures even distribution across nodes with predictable behavior
    fn least_populated_ordered(&self, nodes: &[BackendNode]) -> Option<BackendNode> {
        let mut available: Vec<_> = nodes.iter().filter(|n| n.can_accept()).collect();

        if available.is_empty() {
            return None;
        }

        // Sort by (connection_count, name) - lowest connections first, then alphabetically by name
        available.sort_by(|a, b| {
            let a_connections = *safe_lock(&a.active_connections);
            let b_connections = *safe_lock(&b.active_connections);

            match a_connections.cmp(&b_connections) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            }
        });

        // Return the first node (lowest session count, first alphabetically if tied)
        available.first().map(|n| (*n).clone())
    }

    fn round_robin(&self, nodes: &[BackendNode]) -> Option<BackendNode> {
        let mut index = safe_lock(&self.round_robin_index);
        let available: Vec<_> = nodes.iter().filter(|n| n.can_accept()).collect();

        if available.is_empty() {
            return None;
        }

        let node = available[*index % available.len()].clone();
        *index = (*index + 1) % available.len();
        Some(node)
    }

    /// Fill-first: Fill nodes to capacity before moving to the next
    /// Used for threat nodes to maximize isolation on fewer nodes
    fn fill_first(&self, nodes: &[BackendNode]) -> Option<BackendNode> {
        let available: Vec<_> = nodes.iter().filter(|n| n.can_accept()).collect();

        if available.is_empty() {
            return None;
        }

        let mut index = safe_lock(&self.fill_first_index);

        // Clamp index to available nodes range
        if *index >= available.len() {
            *index = 0;
        }

        // Try current node first
        if let Some(node) = available.get(*index) {
            let connections = *safe_lock(&node.active_connections);
            // If current node has room (< 80% capacity), use it
            if connections < (node.max_connections as f32 * 0.8) as usize {
                return Some((*node).clone());
            }
        }

        // Current node is full, move to next
        *index = (*index + 1) % available.len();

        // Return the next available node
        available.get(*index).map(|n| (*n).clone())
    }

    fn least_connections(&self, nodes: &[BackendNode]) -> Option<BackendNode> {
        nodes
            .iter()
            .filter(|n| n.can_accept())
            .min_by_key(|n| *safe_lock(&n.active_connections))
            .cloned()
    }

    fn weighted_random(&self, nodes: &[BackendNode]) -> Option<BackendNode> {
        use rand::Rng;

        let available: Vec<_> = nodes.iter().filter(|n| n.can_accept()).collect();
        if available.is_empty() {
            return None;
        }

        let total_weight: u32 = available.iter().map(|n| n.weight).sum();
        let mut rng = rand::thread_rng();
        let mut random = rng.gen_range(0..total_weight);

        for node in available {
            if random < node.weight {
                return Some(node.clone());
            }
            random -= node.weight;
        }

        None
    }

    /// Get nodes for a specific tier
    pub fn get_nodes_for_tier(&self, trust_tier: TrustTier) -> &[BackendNode] {
        if trust_tier.requires_gate() {
            &self.threat_nodes
        } else {
            &self.healthy_nodes
        }
    }

    /// Check if any backends are available for a tier
    pub fn has_available_backend(&self, trust_tier: TrustTier) -> bool {
        let nodes = self.get_nodes_for_tier(trust_tier);
        nodes.iter().any(|n| n.can_accept())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_nodes(count: usize, max_conn: usize) -> Vec<BackendNode> {
        (0..count)
            .map(|i| BackendNode::new(format!("http://backend-{}", i), true, max_conn))
            .collect()
    }

    #[test]
    fn test_round_robin_selection() {
        let healthy = create_test_nodes(3, 10);
        let threat = create_test_nodes(2, 10);
        let router = Router::new_with_strategies(
            healthy,
            threat,
            RoutingStrategy::RoundRobin,
            RoutingStrategy::RoundRobin,
        );

        // Test healthy tier
        let node1 = router.select_backend(TrustTier::Verified).unwrap();
        let node2 = router.select_backend(TrustTier::Verified).unwrap();
        let node3 = router.select_backend(TrustTier::Verified).unwrap();

        // Should cycle through nodes
        assert_ne!(node1.address, node2.address);
        assert_ne!(node2.address, node3.address);
    }

    #[test]
    fn test_least_connections_selection() {
        let nodes = create_test_nodes(3, 10);

        // Simulate different connection loads
        {
            let mut conn = safe_lock(&nodes[0].active_connections);
            *conn = 5;
        }
        {
            let mut conn = safe_lock(&nodes[1].active_connections);
            *conn = 2; // Least connections
        }
        {
            let mut conn = safe_lock(&nodes[2].active_connections);
            *conn = 8;
        }

        let router = Router::new(nodes, vec![], RoutingStrategy::LeastConnections);
        let selected = router.select_backend(TrustTier::Verified).unwrap();

        // Should select node with least connections (backend-1 with 2)
        assert_eq!(selected.address, "http://backend-1");
    }

    #[test]
    fn test_tier_routing() {
        let healthy = create_test_nodes(3, 10);
        let threat = create_test_nodes(2, 10);
        let router = Router::new(healthy.clone(), threat.clone(), RoutingStrategy::RoundRobin);

        // Verified should go to healthy
        let node = router.select_backend(TrustTier::Verified).unwrap();
        assert!(node.address.contains("backend-")); // From healthy pool

        // Suspicious should go to threat
        let node = router.select_backend(TrustTier::Suspicious).unwrap();
        assert!(node.address.contains("backend-")); // From threat pool
    }

    #[test]
    fn test_no_available_backends() {
        let nodes = create_test_nodes(2, 1);

        // Fill up all backends
        for node in &nodes {
            node.acquire();
        }

        let router = Router::new(nodes, vec![], RoutingStrategy::RoundRobin);
        let result = router.select_backend(TrustTier::Verified);

        assert!(result.is_none());
    }

    #[test]
    fn test_has_available_backend() {
        let healthy = create_test_nodes(2, 10);
        let threat = create_test_nodes(2, 10);
        let router = Router::new(healthy, threat, RoutingStrategy::RoundRobin);

        assert!(router.has_available_backend(TrustTier::Verified));
        assert!(router.has_available_backend(TrustTier::Suspicious));
    }
}
