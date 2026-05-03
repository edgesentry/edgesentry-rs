use std::net::UdpSocket;

use crate::entity::{Entity, EntityClass, SensorReading, SourceType, Vec2};
use serde::Deserialize;

/// A single entity frame as exported by Unity at 10 Hz via UDP.
///
/// Unity serialises each tick as a JSON object containing a list of entities.
/// The coordinate system is 2D (x, y); the z axis from Unity is discarded.
///
/// Example packet:
/// ```json
/// {
///   "entities": [
///     {"id":"forklift_01","class":"Forklift","x":14.2,"y":31.7,"vx":0.0,"vy":2.1,"timestamp_ms":1714209600123},
///     {"id":"worker_01","class":"Person","x":18.0,"y":31.7,"vx":0.0,"vy":0.0,"timestamp_ms":1714209600123}
///   ]
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct UnityPacket {
    pub entities: Vec<UnityEntityJson>,
}

#[derive(Debug, Deserialize)]
pub struct UnityEntityJson {
    pub id: String,
    pub class: EntityClass,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub confidence: Option<f32>,
}

impl From<UnityEntityJson> for Entity {
    fn from(u: UnityEntityJson) -> Self {
        Entity {
            id: u.id,
            class: u.class,
            position: Vec2::new(u.x, u.y),
            position_z: None,
            velocity: Vec2::new(u.vx, u.vy),
            velocity_z: None,
            timestamp_ms: u.timestamp_ms,
            sensor: Some(SensorReading {
                source_type: SourceType::ComputerVision,
                dimensions: 2,
                detection_confidence: u.confidence,
                position_stddev_m: None,
                position_stddev_z_m: None,
            }),
            computed_confidence: None,
        }
    }
}

/// UDP socket adapter that receives `UnityPacket` JSON from a Unity simulation.
pub struct UnityUdpAdapter {
    socket: UdpSocket,
}

impl UnityUdpAdapter {
    /// Bind to a local UDP address (e.g. `"127.0.0.1:9000"`).
    pub fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self { socket })
    }

    /// Block until a UDP packet arrives, then parse and return the entity list.
    pub fn recv_entities(&self) -> Result<Vec<Entity>, String> {
        let mut buf = [0u8; 65535];
        let (len, _) = self
            .socket
            .recv_from(&mut buf)
            .map_err(|e| format!("UDP recv error: {e}"))?;
        let packet: UnityPacket = serde_json::from_slice(&buf[..len])
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Ok(packet.entities.into_iter().map(Entity::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Vec<Entity> {
        let packet: UnityPacket = serde_json::from_str(json).unwrap();
        packet.entities.into_iter().map(Entity::from).collect()
    }

    #[test]
    fn parse_single_entity() {
        let json = r#"{"entities":[
            {"id":"forklift_01","class":"Forklift","x":14.2,"y":31.7,"vx":0.0,"vy":2.1,"timestamp_ms":1000}
        ]}"#;
        let entities = parse(json);
        assert_eq!(entities.len(), 1);
        let e = &entities[0];
        assert_eq!(e.id, "forklift_01");
        assert_eq!(e.class, crate::entity::EntityClass::Forklift);
        assert!((e.position.x - 14.2).abs() < 1e-5);
        assert!((e.position.y - 31.7).abs() < 1e-5);
        assert!((e.velocity.y - 2.1).abs() < 1e-5);
        assert_eq!(e.timestamp_ms, 1000);
    }

    #[test]
    fn parse_two_entities() {
        let json = r#"{"entities":[
            {"id":"FL-01","class":"Forklift","x":0.0,"y":0.0,"vx":1.4,"vy":0.0,"timestamp_ms":2000},
            {"id":"W-03","class":"Person","x":3.2,"y":0.0,"vx":0.0,"vy":0.0,"timestamp_ms":2000}
        ]}"#;
        let entities = parse(json);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[1].class, crate::entity::EntityClass::Person);
    }

    #[test]
    fn parse_all_entity_classes() {
        for (class_str, expected) in &[
            ("Forklift", crate::entity::EntityClass::Forklift),
            ("ReachStacker", crate::entity::EntityClass::ReachStacker),
            ("TerminalTractor", crate::entity::EntityClass::TerminalTractor),
            ("Vessel", crate::entity::EntityClass::Vessel),
            ("Person", crate::entity::EntityClass::Person),
        ] {
            let json = format!(
                r#"{{"entities":[{{"id":"x","class":"{class_str}","x":0.0,"y":0.0,"vx":0.0,"vy":0.0,"timestamp_ms":0}}]}}"#
            );
            let entities = parse(&json);
            assert_eq!(&entities[0].class, expected, "class {class_str}");
        }
    }

    #[test]
    fn parse_empty_entities_list() {
        let json = r#"{"entities":[]}"#;
        assert!(parse(json).is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result: Result<UnityPacket, _> = serde_json::from_str("{bad json}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unknown_class_returns_error() {
        let json = r#"{"entities":[{"id":"x","class":"Submarine","x":0.0,"y":0.0,"vx":0.0,"vy":0.0,"timestamp_ms":0}]}"#;
        let result: Result<UnityPacket, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ── Unity Scene 2 (maritime demo) packet tests ────────────────────────────
    //
    // ClarusUdpExporter sends {"entities":[{"id":"V-001","class":"Vessel",...}]}.
    // These tests verify the exact packet the Unity vessel scene emits parses
    // correctly into an Entity that the rule engine can evaluate.

    #[test]
    fn parse_vessel_entity_from_unity_packet() {
        // Packet as ClarusUdpExporter would send for V-001 at x=350, y=350
        let json = r#"{"entities":[
            {"id":"V-001","class":"Vessel","x":350.0,"y":350.0,"vx":2.0,"vy":0.0,"timestamp_ms":175000}
        ]}"#;
        let entities = parse(json);
        assert_eq!(entities.len(), 1);
        let e = &entities[0];
        assert_eq!(e.id, "V-001");
        assert_eq!(e.class, crate::entity::EntityClass::Vessel);
        assert!((e.position.x - 350.0).abs() < 1e-4);
        assert!((e.position.y - 350.0).abs() < 1e-4);
        assert!((e.velocity.x - 2.0).abs() < 1e-4, "vx should be 2.0 m/s east");
        assert!(e.velocity.y.abs() < 1e-4, "vy should be 0 (straight east)");
        assert_eq!(e.timestamp_ms, 175_000);
    }

    #[test]
    fn parse_vessel_before_zone_entry() {
        // x=150 — vessel approaching but not yet inside zone
        let json = r#"{"entities":[
            {"id":"V-001","class":"Vessel","x":150.0,"y":350.0,"vx":2.0,"vy":0.0,"timestamp_ms":75000}
        ]}"#;
        let entities = parse(json);
        assert_eq!(entities[0].position.x, 150.0);
    }

    #[test]
    fn parse_scene1_forklift_and_worker_packet() {
        // Packet as Scene 1 ClarusUdpExporter sends — both FL-01 and W-03
        let json = r#"{"entities":[
            {"id":"FL-01","class":"Forklift","x":0.5,"y":0.0,"vx":1.4,"vy":0.0,"timestamp_ms":500},
            {"id":"W-03","class":"Person","x":12.0,"y":0.0,"vx":0.0,"vy":0.0,"timestamp_ms":500}
        ]}"#;
        let entities = parse(json);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].class, crate::entity::EntityClass::Forklift);
        assert_eq!(entities[1].class, crate::entity::EntityClass::Person);
        assert!((entities[0].velocity.x - 1.4).abs() < 1e-4);
    }
}
