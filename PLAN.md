## Default Deployment Profile for Near-Shore Vessels Using VDES

This document extends the existing deployment plan with a second default profile for near-shore vessels. The focus is strictly on tamper prevention and detection for data, device identity, and network-handling/path evidence. It is explicitly noted that analytics, alerting, and regulatory processes are out of scope for this profile.

### Scope
- **Tamper Prevention/Detection**: Ensuring that the data integrity is maintained and tampering is detected effectively.
- **Device Identity**: Establishing a secure identity for the onboard devices to ensure authenticity.
- **Network Handling/Path Evidence**: Providing evidence of the network path taken, ensuring accountability and traceability.

### Components
1. **Onboard Device Agent**: A dedicated agent residing on the vessel to gather data and manage communication.
2. **Onboard Comms Gateway with VDES Adapter**: This includes a communication gateway that connects the onboard device to the VDES (Vessel Data Exchange System) for data transmission.
3. **Store-and-Forward Capability**: In scenarios of weak connectivity, data is stored on the onboard system and forwarded when a connection is re-established.
4. **Shore Office Receiver/Gateway**: A station on land that receives the data sent over VDES, ensuring it is securely processed.

### VDES-Specific Witness Evidence Fields
- Timestamps tracking the exact moment data is sent.
- GPS location data to provide context to the data being sent.
- Unique identifiers for each data transaction to prevent spoofing.

### Handling of Low Bandwidth/High Latency/Retry
Strategies are implemented to manage data transmission challenges effectively:
- **Data Compression**: Reducing the data footprint for transmission.
- **Batch Transmission**: Sending data in batches to optimize network usage.
- **Reliable Delivery Mechanism**: Using acknowledgments to ensure successful data delivery or to trigger retries where necessary.

### Adapter-Based Flexibility
The architecture facilitates adaptability through:
- Use of interchangeable adapters to incorporate future advancements in communication technology without altering existing infrastructure.
- Configurable settings to optimize for various operational conditions based on vessel deployment specifics.