//! LAN HTTP control server.
//!
//! Off by default. When enabled in settings, exposes a tiny JSON API on
//! `http://<lan-ip>:7878/api/*` guarded by a bearer token. Designed so
//! a phone on the same Wi-Fi can drive the lights.
//!
//! Only stubs on non-Windows right now; the Windows path is a thin
//! wrapper around `axum`.

#[cfg(not(target_os = "windows"))]
pub mod imp {
    use anyhow::Result;

    pub async fn start(_port: u16, _token: String) -> Result<()> {
        Err(anyhow::anyhow!(
            "LAN HTTP control is Windows-only in this build; rebuild for Windows"
        ))
    }
}

#[cfg(target_os = "windows")]
pub mod imp {
    use anyhow::Result;

    pub async fn start(port: u16, token: String) -> Result<()> {
        let app = axum::Router::new().route(
            "/api/health",
            axum::routing::get(|| async { "ok" }).layer(axum::middleware::from_fn(
                move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                    let t = token.clone();
                    async move {
                        if req
                            .headers()
                            .get("authorization")
                            .and_then(|h| h.to_str().ok())
                            == Some(&format!("Bearer {t}"))
                        {
                            Ok::<_, axum::http::StatusCode>(next.run(req).await)
                        } else {
                            Err(axum::http::StatusCode::UNAUTHORIZED)
                        }
                    }
                },
            )),
        );

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

pub use imp::start;