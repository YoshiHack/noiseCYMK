//! Diagnostic probes for a single Govee device on the LAN.
//!
//! Used by the UI's "Diagnose" button: given an IP, returns a structured
//! report of which Govee LAN control paths responded and which didn't,
//! so the user can tell whether the device's LAN listener is open, the
//! firmware supports it, or the network path is broken.

use anyhow::Result;
use serde::Serialize;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// What we tried, what we got back.
#[derive(Debug, Serialize, Clone)]
pub struct ProbeReport {
    pub ip: String,
    pub arp_visible: bool,
    pub scan_multicast_received: bool,
    pub unicast_4001_received: bool,
    pub unicast_4003_received: bool,
    pub raw_responses: Vec<String>,
    pub verdict: String,
}

const SCAN_PAYLOAD: &[u8] = b"{\"msg\":{\"cmd\":\"scan\"}}";
const DEVS_STATUS: &[u8] = b"{\"msg\":{\"cmd\":\"devStatus\",\"id\":\"GBK_Unknown\"}}";

/// Send a packet, return Ok(Some(bytes)) if anything came back.
async fn try_send(target: SocketAddr, payload: &[u8], wait: Duration) -> Result<Option<Vec<u8>>> {
    let local: SocketAddr = (std::net::Ipv4Addr::UNSPECIFIED, 0).into();
    let socket = UdpSocket::bind(local).await?;
    let _ = socket.send_to(payload, target).await;
    let mut buf = vec![0u8; 2048];
    match timeout(wait, socket.recv_from(&mut buf)).await {
        Ok(Ok((n, _))) => Ok(Some(buf[..n].to_vec())),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Ok(None), // clean timeout = no reply
    }
}

/// Probe one IP with a battery of Govee LAN control patterns and
/// report what responded. Always returns a `ProbeReport` even on
/// total failure — partial information beats none.
pub async fn diagnose(ip: IpAddr) -> ProbeReport {
    let mut raw_responses = Vec::new();

    // Multicast scan (the way Govee Home discovers devices).
    let mcast_target = (std::net::Ipv4Addr::new(239, 255, 255, 250), 4001).into();
    let scan_multicast_received = match try_send(mcast_target, SCAN_PAYLOAD, Duration::from_millis(1500))
        .await
    {
        Ok(Some(b)) => {
            raw_responses.push(format!("multicast: {}", String::from_utf8_lossy(&b)));
            true
        }
        _ => false,
    };

    // Unicast scan to port 4001 (some routers block multicast between
    // bands even on the same subnet — try direct too).
    let scan_4001 = (ip, 4001).into();
    let unicast_4001_received = match try_send(scan_4001, SCAN_PAYLOAD, Duration::from_millis(1500))
        .await
    {
        Ok(Some(b)) => {
            raw_responses.push(format!("4001/scan: {}", String::from_utf8_lossy(&b)));
            true
        }
        _ => false,
    };

    // Unicast status to port 4003 (control port — what the app uses
    // once discovery is done).
    let ctrl_4003 = (ip, 4003).into();
    let unicast_4003_received = match try_send(ctrl_4003, DEVS_STATUS, Duration::from_millis(1500))
        .await
    {
        Ok(Some(b)) => {
            raw_responses.push(format!("4003/devStatus: {}", String::from_utf8_lossy(&b)));
            true
        }
        _ => false,
    };

    // ARP visibility is checked from the host before calling this — for
    // now we conservatively assume yes since the caller discovered the
    // IP via ARP scan.
    let arp_visible = true;

    let verdict = if unicast_4003_received || unicast_4001_received || scan_multicast_received {
        "Govee LAN listener is open — discovery should work.".to_string()
    } else {
        "No reply on any Govee LAN port. Likely causes:\n\
         • LAN Control is OFF in Govee Home (toggle it off, wait 5s, toggle on).\n\
         • Device firmware doesn't expose LAN for this SKU.\n\
         • Device is on a different Wi-Fi band/VLAN than this PC.\n\
         • Firewall on the PC is blocking outbound UDP to ports 4001/4003."
            .to_string()
    };

    ProbeReport {
        ip: ip.to_string(),
        arp_visible,
        scan_multicast_received,
        unicast_4001_received,
        unicast_4003_received,
        raw_responses,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_report_serializes_to_json() {
        // Diagnose against a non-routable IP — we expect no replies
        // but a structured report should still come back with the
        // verdict explaining what's wrong.
        let r = diagnose("127.0.0.1".parse().unwrap()).await;
        assert_eq!(r.ip, "127.0.0.1");
        assert!(r.arp_visible);
        // Loopback won't respond to Govee scan, so all three should be false.
        assert!(!r.scan_multicast_received);
        assert!(!r.unicast_4001_received);
        assert!(!r.unicast_4003_received);
        assert!(r.verdict.contains("No reply"));
        // Sanity-check it round-trips through serde so the UI can render it.
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"ip\":\"127.0.0.1\""));
        assert!(j.contains("\"verdict\""));
    }
}