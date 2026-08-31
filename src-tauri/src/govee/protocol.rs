//! JSON message types for the Govee LAN protocol.
//!
//! Spec source: Govee's official LAN Control Developer Guide. Examples
//! here are taken verbatim from the spec where possible.

use serde::{Deserialize, Serialize};

/// Command field values (the `cmd` string in the JSON payload).
pub mod cmd {
    pub const SCAN: &str = "scan";
    pub const DEVS_STATUS: &str = "devStatus";
    pub const TURN: &str = "turn";
    pub const TURN_OFF: &str = "turnOff";
    pub const BRIGHTNESS: &str = "brightness";
    pub const COLOR: &str = "color";
    pub const COLOR_WC: &str = "colorwc";
}

/// A request we send to the LAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage {
    pub msg: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum Payload {
    #[serde(rename = "scan")]
    Scan(Scan),
    #[serde(rename = "devStatus")]
    DevStatus(DevStatus),
    #[serde(rename = "turn")]
    Turn(Turn),
    #[serde(rename = "turnOff")]
    TurnOff,
    #[serde(rename = "brightness")]
    Brightness(Brightness),
    #[serde(rename = "color")]
    Color(Color),
    #[serde(rename = "colorwc")]
    ColorWc(ColorWc),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scan {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevStatus {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    pub value: u8, // 0 or 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brightness {
    pub id: String,
    pub value: u8, // 0..=100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub id: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Color with white / color temperature channel. Some Govee SKUs require this
/// instead of plain `color` to drive their RGB+CCT strips properly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorWc {
    pub id: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub colorTemInKelvin: u16,
}

/// A response from the LAN. Discriminated by presence of fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub msg: ResponsePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ResponsePayload {
    #[serde(rename = "scan")]
    Scan(ScanResponse),
    #[serde(rename = "devStatus")]
    DevStatus(DevStatusResponse),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub ip: String,
    pub device: String,   // id
    pub sku: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevStatusResponse {
    pub id: String,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
}

/// Helper constructors.
impl RequestMessage {
    pub fn scan() -> Self {
        Self {
            msg: Payload::Scan(Scan::default()),
        }
    }

    pub fn dev_status(id: impl Into<String>) -> Self {
        Self {
            msg: Payload::DevStatus(DevStatus { id: id.into() }),
        }
    }

    pub fn turn(id: impl Into<String>, on: bool) -> Self {
        Self {
            msg: Payload::Turn(Turn {
                id: id.into(),
                value: if on { 1 } else { 0 },
            }),
        }
    }

    pub fn turn_off(_id: impl Into<String>) -> Self {
        Self {
            msg: Payload::TurnOff,
        }
    }

    pub fn brightness(id: impl Into<String>, pct: u8) -> Self {
        Self {
            msg: Payload::Brightness(Brightness {
                id: id.into(),
                value: pct.min(100),
            }),
        }
    }

    pub fn color(id: impl Into<String>, r: u8, g: u8, b: u8) -> Self {
        Self {
            msg: Payload::Color(Color {
                id: id.into(),
                r,
                g,
                b,
            }),
        }
    }

    pub fn color_wc(
        id: impl Into<String>,
        r: u8,
        g: u8,
        b: u8,
        kelvin: u16,
    ) -> Self {
        Self {
            msg: Payload::ColorWc(ColorWc {
                id: id.into(),
                r,
                g,
                b,
                colorTemInKelvin: kelvin,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_round_trips() {
        let m = RequestMessage::scan();
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(j, r#"{"msg":{"cmd":"scan"}}"#);
        let back: RequestMessage = serde_json::from_str(&j).unwrap();
        match back.msg {
            Payload::Scan(_) => {}
            _ => panic!("scan round-trip did not match"),
        }
    }

    #[test]
    fn turn_on_round_trips() {
        let m = RequestMessage::turn("ABC123", true);
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(
            j,
            r#"{"msg":{"cmd":"turn","id":"ABC123","value":1}}"#
        );
    }

    #[test]
    fn brightness_clamps_to_100() {
        let m = RequestMessage::brightness("ABC", 250);
        match m.msg {
            Payload::Brightness(b) => assert_eq!(b.value, 100),
            _ => panic!(),
        }
    }

    #[test]
    fn color_round_trips() {
        let m = RequestMessage::color("ABC", 255, 128, 0);
        let j = serde_json::to_string(&m).unwrap();
        // The "id" field order matches the struct field order.
        assert_eq!(j, r#"{"msg":{"cmd":"color","id":"ABC","r":255,"g":128,"b":0}}"#);
    }

    #[test]
    fn colorwc_round_trips() {
        let m = RequestMessage::color_wc("ABC", 10, 20, 30, 6500);
        let j = serde_json::to_string(&m).unwrap();
        assert!(j.contains(r#""colorTemInKelvin":6500"#));
    }

    #[test]
    fn scan_response_parses() {
        let j = r#"{"msg":{"cmd":"scan","ip":"192.168.1.50","device":"xx:xx:xx","sku":"H6046"}}"#;
        let r: ResponseMessage = serde_json::from_str(j).unwrap();
        match r.msg {
            ResponsePayload::Scan(s) => {
                assert_eq!(s.sku, "H6046");
                assert_eq!(s.ip, "192.168.1.50");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn unknown_response_is_handled() {
        // Servers sometimes echo back commands we don't recognize.
        let j = r#"{"msg":{"cmd":"something_else","foo":1}}"#;
        let r: ResponseMessage = serde_json::from_str(j).unwrap();
        assert!(matches!(r.msg, ResponsePayload::Unknown));
    }
}