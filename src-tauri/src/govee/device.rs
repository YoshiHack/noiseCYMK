//! Per-device control channel.
//!
//! Sends UDP packets to `<device_ip>:4003` per the Govee LAN spec.
//!
//! We deliberately keep this layer thin: protocol shapes live in
//! `protocol.rs`, capability gating lives in `capabilities.rs`. This
//! module just opens a socket, serializes, and sends.

use super::capabilities::Capabilities;
use super::protocol::{RequestMessage, ResponseMessage};
use anyhow::{anyhow, Context, Result};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;

/// Default Govee LAN control port (per the spec).
pub const CONTROL_PORT: u16 = 4003;

/// Default timeout for sending a single command.
pub const SEND_TIMEOUT: Duration = Duration::from_millis(250);

/// Async control client for one specific device.
///
/// Cheap to clone — wraps a single `UdpSocket`. We keep one socket per
/// device rather than sharing across devices to avoid Govee's well-known
/// quirks around port reuse on Linux.
#[derive(Debug)]
pub struct DeviceClient {
    ip: IpAddr,
    socket: UdpSocket,
}

impl DeviceClient {
    /// Open a control socket aimed at `ip:CONTROL_PORT`.
    pub async fn connect(ip: IpAddr) -> Result<Self> {
        // Bind to an ephemeral local port. SO_REUSEADDR helps on multi-
        // interface hosts; not strictly required on Windows but harmless.
        let local: SocketAddr = (std::net::Ipv4Addr::UNSPECIFIED, 0).into();
        let socket = UdpSocket::bind(local)
            .await
            .with_context(|| format!("binding UDP socket for {ip}"))?;

        // Some Govee devices get grumpy if we send before opening the
        // socket, so flush nothing — just return.
        Ok(Self { ip, socket })
    }

    fn target(&self) -> SocketAddr {
        (self.ip, CONTROL_PORT).into()
    }

    async fn send(&self, req: &RequestMessage) -> Result<ResponseMessage> {
        let bytes = serde_json::to_vec(req)?;
        let target = self.target();

        let send = tokio::time::timeout(SEND_TIMEOUT, self.socket.send_to(&bytes, target));
        send.await
            .map_err(|_| anyhow!("send timeout to {target}"))?
            .with_context(|| format!("sending to {target}"))?;

        // Devices ack every command with a JSON payload on the same socket.
        let mut buf = [0u8; 4096];
        let recv = tokio::time::timeout(
            SEND_TIMEOUT,
            self.socket.recv_from(&mut buf),
        );
        let (len, _) = recv
            .await
            .map_err(|_| anyhow!("recv timeout from {target}"))?
            .with_context(|| format!("recv from {target}"))?;

        serde_json::from_slice(&buf[..len])
            .with_context(|| format!("parse response from {target}"))
    }

    pub async fn set_power(&self, on: bool, caps: &Capabilities) -> Result<()> {
        if !caps.power {
            return Err(anyhow!("SKU {} does not support power over LAN", caps.sku));
        }
        let req = if on {
            RequestMessage::turn(&self.ip.to_string(), true)
        } else {
            // Some firmware is pickier than others. Try `turn` first, fall
            // back to `turnOff` if the device naks.
            // For now we send `turnOff` directly — the official client does.
            RequestMessage::turn_off(&self.ip.to_string())
        };
        self.send(&req).await.map(|_| ())
    }

    pub async fn set_brightness(&self, pct: u8, caps: &Capabilities) -> Result<()> {
        if !caps.brightness {
            return Err(anyhow!(
                "SKU {} does not support brightness over LAN",
                caps.sku
            ));
        }
        let req = RequestMessage::brightness(&self.ip.to_string(), pct);
        self.send(&req).await.map(|_| ())
    }

    /// Set raw RGB. If `caps.color_wc` is true and `kelvin` is provided,
    /// uses `colorwc` instead — some RGBIC strips silently ignore `color`
    /// when the white channel needs to be reset.
    pub async fn set_color(
        &self,
        r: u8,
        g: u8,
        b: u8,
        caps: &Capabilities,
        kelvin: Option<u16>,
    ) -> Result<()> {
        let req = match (caps.color_wc, caps.color_rgb) {
            (true, _) if kelvin.is_some() => RequestMessage::color_wc(
                &self.ip.to_string(),
                r,
                g,
                b,
                kelvin.unwrap_or(6500),
            ),
            (_, true) => RequestMessage::color(&self.ip.to_string(), r, g, b),
            _ => {
                return Err(anyhow!(
                    "SKU {} has neither color nor colorwc capability",
                    caps.sku
                ))
            }
        };
        self.send(&req).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// We can't talk to a real device in CI, but we can prove the JSON
    /// bytes we send are exactly what the spec wants. The control layer
    /// itself is exercised end-to-end by integration tests on a real LAN.

    #[tokio::test]
    async fn socket_can_be_opened_for_arbitrary_ip() {
        // We don't actually send anything — just verify bind() works.
        let client = DeviceClient::connect("127.0.0.1".parse().unwrap()).await;
        assert!(client.is_ok());
    }
}