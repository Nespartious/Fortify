# Community Network

## Purpose

The community network is an **optional** decentralized discovery mechanism that allows Fortify deployments to share orchestrator addresses without centralized coordination.

## Critical Properties

- **Discovery ≠ Trust**: Finding an orchestrator doesn't bypass verification
- **Opt-In**: Deployments choose whether to participate
- **No Bypass**: All traffic still goes through Gate
- **Anonymity Preserved**: No identity linkage

## Architecture

### Seed Registry
- Signed list of community participant addresses
- Distributed via Tor or other anonymous channels
- Updated periodically by maintainers
- Contains:
  - Onion addresses of participating deployments
  - Public keys for verification
  - Optional metadata (capacity, region, etc.)

### Daisy-Chain Discovery
1. User accesses known seed orchestrator
2. Orchestrator provides `/community` page
3. Page lists other participating orchestrators
4. User selects alternative entry point
5. Connection to new orchestrator starts at Gate

### Community Page
- Static HTML only
- Lists active orchestrators
- Displays health/capacity indicators
- No JavaScript
- No direct tunneling

## Participation Levels

### Level 0: Standalone
- Default mode
- No community participation
- Orchestrator addresses distributed out-of-band

### Level 1: Consumer
- Fetches community registry
- Displays other orchestrators on `/community`
- Does not advertise own addresses

### Level 2: Provider
- Advertises own orchestrators in community
- Signs registry entries with deployment key
- May serve community page

### Level 3: Seed
- Maintains authoritative registry
- Signs and publishes updates
- High-availability commitment
- Trusted by community

## Registry Format

```toml
[[community]]
onion_address = "abc123...xyz.onion"
public_key = "ed25519:..."
capacity = "medium"
region = "neutral"
signature = "..."

[[community]]
onion_address = "def456...uvw.onion"
public_key = "ed25519:..."
capacity = "high"
region = "neutral"
signature = "..."
```

## Security Considerations

### Trust Model
- **Registry Signing**: Prevents injection attacks
- **No Single Point of Failure**: Multiple seeds
- **Verification Required**: Signatures checked on fetch
- **Expiration**: Registry entries have TTL

### Risks
- **Sybil Attacks**: Attacker runs many orchestrators
  - Mitigation: Proof-of-work or vouching system
- **Deanonymization**: Correlation across deployments
  - Mitigation: Each deployment remains independent
- **Registry Poisoning**: Malicious entries
  - Mitigation: Signature verification, reputation

### Privacy
- **No Persistent Identity**: Each orchestrator is disposable
- **No Metadata Leakage**: Minimal registry information
- **Optional Participation**: Deployments can leave anytime

## Implementation Strategy

### Phase 8 Deliverables
1. Registry structure definition
2. Signature verification logic
3. `/community` static page generation
4. Registry fetch and validation
5. Configuration for participation level

### Non-Goals
- Reputation systems (out of scope)
- Payment/incentive mechanisms
- Automated mesh networking
- Cross-deployment trust sharing

## Operational Guidelines

### For Seed Operators
- Maintain high availability
- Sign registry updates regularly
- Vet new participants (manual process)
- Rotate signing keys periodically

### For Participants
- Choose participation level carefully
- Monitor orchestrator health
- Update registry information when rotating
- Leave gracefully (stop advertising)

### For Users
- Verify registry signatures
- Diversify across multiple orchestrators
- Don't trust community listing as endorsement
- Still complete Gate verification

## Configuration Options

```toml
[community]
enabled = false                        # Participate in community network
mode = "standalone"                    # standalone | consumer | provider | seed
registry_url = "http://...onion/registry.toml"
update_interval = 3600                 # Seconds between registry fetches
signing_key_path = "/path/to/key"      # For providers/seeds
```

## Future Enhancements (Out of Scope for Initial)
- Proof-of-work vouching system
- Distributed hash table (DHT) for registry
- Anonymous reputation metrics
- Cross-deployment load balancing
