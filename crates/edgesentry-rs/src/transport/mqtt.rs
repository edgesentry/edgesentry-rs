//! MQTT ingest transport scaffold.
//!
//! This module reserves the configuration types for a future MQTT-based ingest
//! transport.  Full protocol implementation is tracked in a separate issue.

/// Quality-of-service level for MQTT message delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MqttQos {
    /// At most once delivery (fire and forget).
    #[default]
    AtMostOnce,
    /// At least once delivery (acknowledged).
    AtLeastOnce,
    /// Exactly once delivery (four-way handshake).
    ExactlyOnce,
}

/// Configuration for an MQTT ingest endpoint.
///
/// Pass this to the (forthcoming) MQTT serve function once the transport is
/// fully implemented.
#[derive(Debug, Clone)]
pub struct MqttIngestConfig {
    /// MQTT broker host or IP.
    pub broker_host: String,
    /// MQTT broker port (default 1883, or 8883 for TLS).
    pub broker_port: u16,
    /// Topic to subscribe to for incoming audit records.
    pub topic: String,
    /// MQTT client identifier.
    pub client_id: String,
    /// Quality-of-service level for inbound messages.
    pub qos: MqttQos,
}

impl MqttIngestConfig {
    /// Construct a configuration with default port 1883.
    pub fn new(
        broker_host: impl Into<String>,
        topic: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            broker_host: broker_host.into(),
            broker_port: 1883,
            topic: topic.into(),
            client_id: client_id.into(),
            qos: MqttQos::default(),
        }
    }
}
