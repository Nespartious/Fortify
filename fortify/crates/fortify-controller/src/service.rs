use crate::ControllerError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

/// Service type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Orchestrator,
    Gate,
    HttpProxy,
    Node,
}

impl ServiceType {
    pub fn binary_name(&self) -> &str {
        match self {
            ServiceType::Orchestrator => "fortify-orchestrator",
            ServiceType::Gate => "fortify-gate",
            ServiceType::HttpProxy => "fortify-http",
            ServiceType::Node => "fortify-node",
        }
    }
}

/// Service status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Starting,
    Running,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSnapshot {
    pub id: String,
    pub service_type: ServiceType,
    pub status: ServiceStatus,
    pub restart_count: usize,
    pub bind_addr: Option<String>,
    pub mode: Option<String>,
    pub onion_address: Option<String>,
}

/// Service instance
pub struct ServiceInstance {
    pub id: String,
    pub service_type: ServiceType,
    pub status: ServiceStatus,
    pub process: Option<Child>,
    pub started_at: Instant,
    pub restart_count: usize,
    pub env_vars: Vec<String>,
}

impl ServiceInstance {
    pub fn new(id: String, service_type: ServiceType, env_vars: Vec<String>) -> Self {
        Self {
            id,
            service_type,
            status: ServiceStatus::Starting,
            process: None,
            started_at: Instant::now(),
            restart_count: 0,
            env_vars,
        }
    }

    /// Check if process is still running
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => false, // Process exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking status
            }
        } else {
            false
        }
    }

    /// Stop the service
    pub fn stop(&mut self) -> Result<(), ControllerError> {
        if let Some(mut child) = self.process.take() {
            child.kill().map_err(|e| {
                ControllerError::ServiceError(format!("Failed to kill process: {}", e))
            })?;

            self.status = ServiceStatus::Stopped;
        }
        Ok(())
    }

    fn snapshot(&self) -> ServiceSnapshot {
        ServiceSnapshot {
            id: self.id.clone(),
            service_type: self.service_type,
            status: self.status.clone(),
            restart_count: self.restart_count,
            bind_addr: self.bind_addr_hint(),
            mode: self.mode_hint(),
            onion_address: self.onion_hint(),
        }
    }

    fn bind_addr_hint(&self) -> Option<String> {
        let keys: &[&str] = match self.service_type {
            ServiceType::Gate => &["GATE_BIND_ADDR"],
            ServiceType::HttpProxy => &["PROXY_BIND_ADDR"],
            ServiceType::Orchestrator => &["ORCH_BIND_ADDR"],
            ServiceType::Node => &["BIND_ADDR"],
        };
        self.lookup_env(keys)
    }

    fn mode_hint(&self) -> Option<String> {
        self.lookup_env(&["NODE_MODE"])
    }
    
    fn onion_hint(&self) -> Option<String> {
        self.lookup_env(&["ONION_ADDRESS"])
    }

    fn lookup_env(&self, keys: &[&str]) -> Option<String> {
        for entry in &self.env_vars {
            if let Some((key, value)) = entry.split_once('=') {
                if keys.iter().any(|candidate| candidate == &key) {
                    return Some(value.to_string());
                }
            }
        }
        None
    }
}

/// Service manager
pub struct ServiceManager {
    services: HashMap<String, ServiceInstance>,
    next_id: usize,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            next_id: 0,
        }
    }

    /// Spawn a new service
    pub fn spawn(
        &mut self,
        service_type: ServiceType,
        name: String,
        env_vars: Vec<String>,
    ) -> Result<String, ControllerError> {
        let id = format!("{}-{}", name, self.next_id);
        self.next_id += 1;

        let mut instance = ServiceInstance::new(id.clone(), service_type, env_vars.clone());

        // Find the binary path
        let binary_path = Self::find_binary(service_type.binary_name())?;
        tracing::debug!("Found binary for {}: {}", service_type.binary_name(), binary_path.display());

        // Build command
        let mut cmd = Command::new(&binary_path);
        // Inherit stdout/stderr so child logs appear in controller output
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        // Add environment variables
        for env in &env_vars {
            if let Some((key, value)) = env.split_once('=') {
                cmd.env(key, value);
            }
        }

        // Spawn process
        let child = cmd.spawn().map_err(|e| {
            ControllerError::ServiceError(format!(
                "Failed to spawn {}: {}",
                service_type.binary_name(),
                e
            ))
        })?;

        instance.process = Some(child);
        instance.status = ServiceStatus::Running;

        self.services.insert(id.clone(), instance);

        tracing::info!("Spawned {} service: {}", service_type.binary_name(), id);

        Ok(id)
    }

    /// Find binary in target directory, next to current exe, or in PATH
    fn find_binary(name: &str) -> Result<PathBuf, ControllerError> {
        // Check relative to current exe first (most common case when running from target/)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let sibling = dir.join(name);
                if sibling.exists() {
                    tracing::debug!("Found {} next to current exe: {}", name, sibling.display());
                    return Ok(sibling);
                }
            }
        }

        // Check target/release (when running from project root)
        let release = PathBuf::from(format!("target/release/{}", name));
        if release.exists() {
            tracing::debug!("Found {} in target/release", name);
            return Ok(release.canonicalize().unwrap_or(release));
        }

        // Check target/debug
        let debug = PathBuf::from(format!("target/debug/{}", name));
        if debug.exists() {
            tracing::debug!("Found {} in target/debug", name);
            return Ok(debug.canonicalize().unwrap_or(debug));
        }

        // Check common installation paths
        let paths = [
            PathBuf::from(format!("/usr/local/bin/{}", name)),
            PathBuf::from(format!("/usr/bin/{}", name)),
        ];
        
        // Also check ~/.cargo/bin
        if let Ok(home) = std::env::var("HOME") {
            let cargo_bin = PathBuf::from(format!("{}/.cargo/bin/{}", home, name));
            if cargo_bin.exists() {
                tracing::debug!("Found {} in ~/.cargo/bin", name);
                return Ok(cargo_bin);
            }
        }
        
        for p in &paths {
            if p.exists() {
                tracing::debug!("Found {} at {}", name, p.display());
                return Ok(p.clone());
            }
        }

        // Fall back to PATH lookup - Command::new will resolve it
        // But first check if it's actually in PATH using `which`
        if let Ok(output) = std::process::Command::new("which")
            .arg(name)
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    tracing::debug!("Found {} in PATH: {}", name, path);
                    return Ok(PathBuf::from(path));
                }
            }
        }

        Err(ControllerError::ServiceError(format!(
            "Binary '{}' not found. Searched: current exe dir, target/release, target/debug, /usr/local/bin, /usr/bin, ~/.cargo/bin, PATH",
            name
        )))
    }

    /// Check health of all services
    pub async fn check_health(&mut self) -> Vec<String> {
        let mut failed = Vec::new();

        for (id, instance) in self.services.iter_mut() {
            if !instance.is_alive() && instance.status == ServiceStatus::Running {
                instance.status = ServiceStatus::Failed;
                failed.push(id.clone());
            }
        }

        failed
    }

    /// Restart a service
    pub async fn restart(&mut self, service_id: &str) -> Result<(), ControllerError> {
        let service = self
            .services
            .get_mut(service_id)
            .ok_or_else(|| ControllerError::ServiceError("Service not found".to_string()))?;

        // Stop existing process
        service.stop()?;

        // Increment restart count
        service.restart_count += 1;

        // Respawn
        let service_type = service.service_type;
        let env_vars = service.env_vars.clone();

        let mut cmd = Command::new(service_type.binary_name());
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        for env in &env_vars {
            if let Some((key, value)) = env.split_once('=') {
                cmd.env(key, value);
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| ControllerError::ServiceError(format!("Failed to restart: {}", e)))?;

        service.process = Some(child);
        service.status = ServiceStatus::Running;
        service.started_at = Instant::now();

        Ok(())
    }

    /// Stop a service
    pub async fn stop(&mut self, service_id: &str) -> Result<(), ControllerError> {
        let service = self
            .services
            .get_mut(service_id)
            .ok_or_else(|| ControllerError::ServiceError("Service not found".to_string()))?;

        service.stop()?;
        Ok(())
    }

    /// Shutdown all services
    pub async fn shutdown_all(&mut self) -> Result<(), ControllerError> {
        let ids: Vec<String> = self.services.keys().cloned().collect();

        for id in ids {
            self.stop(&id).await?;
        }

        self.services.clear();
        Ok(())
    }

    /// Count services by type
    pub fn count_by_type(&self, service_type: ServiceType) -> usize {
        self.services
            .values()
            .filter(|s| s.service_type == service_type && s.status == ServiceStatus::Running)
            .count()
    }

    /// Snapshot service metadata for diagnostics
    pub fn snapshots(&self) -> Vec<ServiceSnapshot> {
        self.services.values().map(|svc| svc.snapshot()).collect()
    }

    /// Get running service count
    pub fn running_count(&self) -> usize {
        self.services
            .values()
            .filter(|s| s.status == ServiceStatus::Running)
            .count()
    }

    /// Get failed service count
    pub fn failed_count(&self) -> usize {
        self.services
            .values()
            .filter(|s| s.status == ServiceStatus::Failed)
            .count()
    }

    /// Select a service for removal (oldest running service of type)
    pub fn select_for_removal(&self, service_type: ServiceType) -> Option<String> {
        self.services
            .iter()
            .filter(|(_, s)| s.service_type == service_type && s.status == ServiceStatus::Running)
            .min_by_key(|(_, s)| s.started_at)
            .map(|(id, _)| id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_binary_names() {
        assert_eq!(
            ServiceType::Orchestrator.binary_name(),
            "fortify-orchestrator"
        );
        assert_eq!(ServiceType::Gate.binary_name(), "fortify-gate");
        assert_eq!(ServiceType::HttpProxy.binary_name(), "fortify-http");
        assert_eq!(ServiceType::Node.binary_name(), "fortify-node");
    }

    #[test]
    fn test_service_manager_creation() {
        let manager = ServiceManager::new();
        assert_eq!(manager.running_count(), 0);
        assert_eq!(manager.failed_count(), 0);
    }

    #[test]
    fn test_count_by_type() {
        let manager = ServiceManager::new();

        assert_eq!(manager.count_by_type(ServiceType::Orchestrator), 0);
        assert_eq!(manager.count_by_type(ServiceType::Gate), 0);
    }

    #[tokio::test]
    async fn test_shutdown_all() {
        let mut manager = ServiceManager::new();

        // Shutdown should succeed even with no services
        assert!(manager.shutdown_all().await.is_ok());
    }
}
