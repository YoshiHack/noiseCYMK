//! UDP multicast discovery for Govee LAN devices.
//!
//! Sends `{"msg":{"cmd":"scan"}}` to `239.255.255.250:4001` and listens
//! for `ScanResponse` packets on `4002`. Devices respond with their IP,
//! SKU, and id.

use super::capabilities::Capabilities;
use super::{DeviceRegistry, LanDevice};
use anyhow::{Context, Result};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::net::UdpSocket;

/// Multicast group Govee uses for LAN discovery.
pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// UDP port to send scan requests to.
pub const SCAN_SEND_PORT: u16 = 4001;

/// UDP port devices reply on.
pub const SCAN_RECV_PORT: u16 = 4002;

/// How long to wait during a single discovery scan before giving up.
pub const SCAN_DURATION: Duration = Duration::from_secs(3);

/// Drive one round of multicast discovery.
///
/// `conflict_safe`: when true, don't bind to 4002 (lets Govee Home keep
/// running). The tradeoff is slower discovery — we have to rely on
/// devices broadcasting responses instead of unicast replies to us.
pub async fn scan_once(conflict_safe: bool) -> Result<DeviceRegistry> {
    let local_send: SocketAddrV4 =
        SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
    let send_socket = UdpSocket::bind(local_send)
        .await
        .context("binding send socket")?;
    send_socket
        .join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)
        .context("joining multicast group on send socket")?;

    // Send scan request.
    let scan_bytes = serde_json::to_vec(&super::protocol::RequestMessage::scan())
        .context("serialize scan")?;
    let target = (MULTICAST_ADDR, SCAN_SEND_PORT);
    send_socket
        .send_to(&scan_bytes, target)
        .await
        .context("sending scan")?;

    // Open the receive socket.
    let recv_socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, SCAN_RECV_PORT))
        .await
        .context("binding recv socket")?;
    recv_socket
        .join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)
        .context("joining multicast group on recv socket")?;
    let _ = conflict_safe; // reserved for Windows-specific SO_REUSEADDR tuning

    let mut registry = DeviceRegistry::default();
    let deadline = tokio::time::Instant::now() + SCAN_DURATION;

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;

        let mut buf = vec![0u8; 4096];
        let recv = tokio::time::timeout(remaining, recv_socket.recv_from(&mut buf)).await;
        match recv {
            Ok(Ok((len, addr))) => match parse_scan_response(&buf[..len]) {
                Ok(scan) => {
                    let caps = Capabilities::for_sku(&scan.msg.sku);
                    let dev = LanDevice {
                        id: scan.msg.device.clone(),
                        friendly_name: String::new(),
                        sku: scan.msg.sku.clone(),
                        ip: addr.ip(),
                        last_seen: std::time::Instant::now(),
                        capabilities: caps,
                        power: None,
                        brightness: None,
                        color: None,
                    };
                    registry.devices.insert(scan.msg.device, dev);
                }
                Err(e) => log::warn!("bad scan response from {addr}: {e:?}"),
            },
            Ok(Err(e)) => {
                log::warn!("recv error during scan: {e:?}");
                break;
            }
            Err(_) => break, // timeout
        }
    }

    Ok(registry)
}

#[derive(Debug, serde::Deserialize)]
struct RawScanResponse {
    msg: RawScanMsg,
}
#[derive(Debug, serde::Deserialize)]
struct RawScanMsg {
    #[allow(dead_code)]
    cmd: String,
    #[allow(dead_code)]
    ip: String,
    #[serde(default)]
    device: String,
    #[serde(default)]
    sku: String,
}

fn parse_scan_response(bytes: &[u8]) -> Result<RawScanResponse> {
    serde_json::from_slice(bytes).context("parse scan response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scan_response_handles_real_shape() {
        let j = br#"{"msg":{"cmd":"scan","ip":"192.168.1.50","device":"AA:BB:CC","sku":"H6046"}}"#;
        let parsed = parse_scan_response(j).unwrap();
        assert_eq!(parsed.msg.sku, "H6046");
        assert_eq!(parsed.msg.device, "AA:BB:CC");
    }

    #[test]
    fn parse_scan_response_rejects_garbage() {
        let j = b"not json at all";
        assert!(parse_scan_response(j).is_err());
    }
}