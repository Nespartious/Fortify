# Scaling Model

## Philosophy

Fortify scales **within resource limits**, not to infinite capacity. The goal is sustainable protection, not maximum throughput.

## Scaling Dimensions

### 1. Orchestrators (Horizontal)
**Purpose**: Distribute discovery and absorb attacks

- **Scaling Strategy**:
  - Spawn multiple public-facing orchestrators
  - Each has unique onion address
  - Advertise via different channels
  - Rotate proactively (burn before compromise)

- **Resource Cost**: Low
  - Minimal processing (routing only)
  - No state storage
  - Small memory footprint

- **Limits**:
  - Tor descriptor publication rate
  - Network bandwidth
  - Management overhead

- **Trigger**: 
  - Threat detection
  - Time-based rotation
  - Traffic threshold

### 2. Gate (Fixed Capacity)
**Purpose**: Intentionally slow verification

- **Scaling Strategy**: 
  - **Does NOT scale horizontally**
  - Fixed worker pool
  - Rate limited by design
  - Backpressure to orchestrators

- **Resource Cost**: High per verification
  - CPU for captcha generation
  - CPU for PoW validation
  - Memory for challenge state

- **Limits**:
  - Deliberate bottleneck
  - Typically 1-10 verifications/second
  - Configuration sets max capacity

- **Philosophy**: Verification is expensive by design

### 3. HTTP Proxy (Vertical + Limited Horizontal)
**Purpose**: Fast path for verified sessions

- **Scaling Strategy**:
  - Worker pool scales with CPU cores
  - Optional multiple instances
  - Minimal per-request overhead

- **Resource Cost**: Low per request
  - Token validation only
  - No buffering
  - Streaming proxy

- **Limits**:
  - Connection count
  - Network bandwidth
  - Backend capacity

- **Trigger**:
  - CPU utilization
  - Connection queue depth
  - Response time degradation

### 4. Nodes (Horizontal by Threat Level)
**Purpose**: Traffic separation and forwarding

- **Scaling Strategy**:
  - Separate pools for Healthy vs Threat
  - Scale each pool independently
  - Healthy pool gets more resources

- **Resource Cost**: Medium
  - Connection state
  - Request inspection (Threat mode)
  - Forwarding overhead

- **Limits**:
  - Real service capacity
  - Network bandwidth
  - Memory for connection state

- **Trigger**:
  - Traffic distribution (Healthy vs Threat)
  - Real service backpressure
  - Connection timeouts

## Resource Governance

### CPU
- Fixed allocation per component
- Gate gets lowest priority
- HTTP Proxy gets highest priority
- Controller monitors and adjusts

### Memory
- Hard caps per component
- No unbounded queues
- Connection limits enforced
- OOM prevention via backpressure

### Network
- Rate limiting at multiple layers
- Connection pooling
- Backpressure to upstream
- Graceful degradation

### Storage
- Minimal disk usage
- No persistent logs by default
- Temporary files cleaned aggressively
- No database requirement

## Scaling Decisions

### When to Scale Up

#### Add Orchestrator Mirror
- Current mirrors compromised
- Traffic concentration risk
- Time-based rotation schedule
- New distribution channel available

#### Add Healthy Node
- Sustained high verified traffic
- Response time degradation
- Real service capacity available
- CPU utilization < 80%

#### Add Threat Node
- Suspicious session ratio high
- Threat queue growing
- Inspection backlog

### When to Scale Down

#### Burn Orchestrator
- Compromise detected
- Scheduled rotation
- Low traffic (consolidate)

#### Remove Node
- Low traffic sustained
- Resource constraints
- Health check failures

### When to Reject

#### Gate Capacity Reached
- Verification queue full
- CPU exhausted
- Return HTTP 503

#### Backend Unavailable
- Real service down
- Network partition
- Return generic error

## Deployment Profiles

### Minimal (Single VPS)
- 1 Orchestrator
- 1 Gate instance
- 1 HTTP Proxy
- 2 Nodes (1 Healthy, 1 Threat)
- 1 Controller

**Capacity**: 10-50 concurrent verified users

### Standard (Dedicated Server)
- 3 Orchestrators (rotated)
- 1 Gate instance
- 2 HTTP Proxy instances
- 8 Nodes (6 Healthy, 2 Threat)
- 1 Controller

**Capacity**: 100-500 concurrent verified users

### Hardened (Multiple Hosts)
- 5+ Orchestrators (aggressive rotation)
- 2 Gate instances (separate hosts)
- 4 HTTP Proxy instances
- 16 Nodes (12 Healthy, 4 Threat)
- 2 Controllers (failover)

**Capacity**: 500-2000 concurrent verified users

## Auto-Scaling Logic

### Controller Responsibilities
1. Monitor resource utilization
2. Track request queue depths
3. Observe response times
4. Count active sessions by trust tier
5. Make scaling decisions
6. Spawn/kill component instances
7. Update routing tables

### Heuristics
- **Scale up**: Sustained high load (>80%) for >5 minutes
- **Scale down**: Low load (<20%) for >15 minutes
- **Emergency**: Immediate rejection when >95% capacity
- **Burn**: Compromise indicators or scheduled rotation

### Safe Shutdown
1. Stop accepting new connections
2. Drain existing connections (timeout: 30s)
3. Persist essential state (if any)
4. Notify Controller
5. Exit cleanly

## Limitations and Trade-offs

### By Design
- **No infinite scale**: System has hard capacity limits
- **Slow onboarding**: Gate is intentionally slow
- **Resource bounded**: No cloud auto-scaling
- **Availability secondary**: Will reject traffic to preserve secrecy

### Operational
- **Manual capacity planning**: Operator sets resource limits
- **No load prediction**: Reactive, not predictive
- **Single-host bias**: Multi-host deployment complex
- **Tor latency**: Inherent 3-7 hop latency

## Monitoring Metrics

### Per-Component
- CPU usage
- Memory usage
- Connection count
- Request rate
- Error rate
- Response time

### System-Wide
- Total verified sessions
- Trust tier distribution
- Orchestrator burn rate
- Gate rejection rate
- Backend health

### Alerting Thresholds
- CPU >90%: Warning
- Memory >90%: Critical
- Gate rejections >50%: Review capacity
- Backend errors >5%: Check real service
