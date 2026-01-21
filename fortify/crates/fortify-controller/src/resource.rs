use sysinfo::System;

/// Resource monitor for tracking system utilization
pub struct ResourceMonitor {
    system: System,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self { system }
    }

    /// Update resource information
    pub fn update(&mut self) {
        self.system.refresh_cpu();
        self.system.refresh_memory();
    }

    /// Get CPU usage percentage (0-100)
    pub fn cpu_usage_percent(&self) -> f32 {
        self.system.global_cpu_info().cpu_usage()
    }

    /// Get total memory in MB
    pub fn memory_total_mb(&self) -> u64 {
        self.system.total_memory() / 1024 / 1024
    }

    /// Get used memory in MB
    pub fn memory_used_mb(&self) -> u64 {
        self.system.used_memory() / 1024 / 1024
    }

    /// Get available memory in MB
    pub fn memory_available_mb(&self) -> u64 {
        self.system.available_memory() / 1024 / 1024
    }

    /// Get memory usage percentage (0-100)
    pub fn memory_usage_percent(&self) -> f32 {
        (self.memory_used_mb() as f32 / self.memory_total_mb() as f32) * 100.0
    }

    /// Check if resources are available for scaling up
    pub fn can_scale_up(&self) -> bool {
        self.cpu_usage_percent() < 80.0 && self.memory_usage_percent() < 80.0
    }

    /// Check if resources are critically low
    pub fn is_resource_critical(&self) -> bool {
        self.cpu_usage_percent() > 90.0 || self.memory_usage_percent() > 90.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_creation() {
        let monitor = ResourceMonitor::new();

        // Basic sanity checks
        assert!(monitor.memory_total_mb() > 0);
        assert!(monitor.cpu_usage_percent() >= 0.0);
    }

    #[test]
    fn test_memory_calculations() {
        let mut monitor = ResourceMonitor::new();
        monitor.update();

        let total = monitor.memory_total_mb();
        let used = monitor.memory_used_mb();
        let available = monitor.memory_available_mb();

        assert!(total > 0);
        assert!(used <= total);
        assert!(available <= total);
    }

    #[test]
    fn test_memory_usage_percent() {
        let monitor = ResourceMonitor::new();
        let usage = monitor.memory_usage_percent();

        assert!(usage >= 0.0);
        assert!(usage <= 100.0);
    }

    #[test]
    fn test_resource_thresholds() {
        let mut monitor = ResourceMonitor::new();
        monitor.update();

        // Just verify these methods don't panic
        let _ = monitor.can_scale_up();
        let _ = monitor.is_resource_critical();
    }
}
