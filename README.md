# NoiseCMYK

CMYK-mapped ambient lighting for Govee devices over LAN — a from-scratch alternative to SignalRGB that doesn't fight your Govee Home app.

**Why:** SignalRGB's Govee integration is unreliable. The LAN control toggle hides behind firmware updates, the Govee Home app and SignalRGB fight for the same UDP socket, and most RGBIC strips get treated as one zone because Govee's LAN API only exposes one color at a time. NoiseCMYK is small (~10 MB installer, ~50 MB RAM), talks directly to your lights on the local network, and ships screen-color sampling that actually works (DXGI Desktop Duplication on Windows, `xcap` on Linux/macOS, gamma-correct averaging).

The "CMYK" in the name is the screen-sync trick: each light reacts to a different printer channel of the on-screen image. Light bars = Cyan, strip = Magenta, backlight = Yellow, fourth device = Key/black. Four lights become a printer.

**No cloud, no subscription, no telemetry.** Your lights stay on your network.

## Status

Phase 0 / Phase 1 / Phase 2 complete:

- ✅ Tauri 2 + Rust + React + TypeScript + Tailwind project scaffold
- ✅ Govee LAN protocol — JSON message types, round-trip tests against spec shapes
- ✅ UDP multicast discovery on `239.255.255.250:4001/4002`
- ✅ Per-device control client (`color`, `brightness`, `turn`, `turnOff`, `colorwc`)
- ✅ Per-SKU capability map for H6046, H610A, H6609
- ✅ Gamma-correct color-zone sampler (the #1 ambient-light bug)
- ✅ Effects engine: solid, breathing, rainbow, screen_sync
- ✅ Tauri IPC + React frontend (devices, colors, brightness, power, effects)
- ✅ **Real DXGI Desktop Duplication capture (Windows)** + `xcap` fallback (Linux/macOS)
- ✅ **CMYK channel decomposition** — each device gets a different printer channel
- ✅ Live screen-sync scheduler (tokio task, configurable FPS)
- ⏳ LAN HTTP control server (opt-in, bearer-token) — Phase 5

Run `cargo test -p noiseCYMK` from `src-tauri/` — 30+ tests pass on Linux for the cross-platform core. Windows-only DXGI bits and Linux/macOS xcap bits are `#[cfg]`-gated so the same `cargo check` runs on your CI runner and dev machine.

## Devices supported (out of the box)

| SKU | Type | LAN capability |
|---|---|---|
| H6046 | RGBIC LED light bars | Single-zone color, brightness, power |
| H610A | RGBIC LED strip | Color, brightness, power, colorwc |
| H6609 | TV Backlight 3 Lite (camera + RGBIC strip) | Color, brightness, power, colorwc |

Other Govee SKUs will be discovered and fall back to conservative defaults (color + brightness + power). Add new SKUs to `src-tauri/src/govee/capabilities.rs` as you characterize them.

## Building from source

### On your Windows box (recommended for development)

Prereqs: Rust (stable), Node 20+, Visual Studio Build Tools (C++ workload), WebView2 runtime.

```bash
git clone https://github.com/jacksonhughes/lightsync
cd lightsync
npm install
npm run tauri dev      # hot-reload dev
npm run tauri build    # produces MSI + NSIS in src-tauri/target/release/bundle/
```

### Cross-compiled MSI via GitHub Actions

Every push to `main` and every `v*` tag triggers `.github/workflows/build.yml`, which builds and uploads signed (or unsigned) MSI + NSIS artifacts. Download from the Actions run page.

## How it differs from SignalRGB

| | SignalRGB | LightSync |
|---|---|---|
| Talks to Govee via | Cloud-friendly LAN API | Direct LAN API, no cloud |
| Conflicts with Govee Home | Yes (UDP 4002 bind race) | Optional conflict-safe mode |
| Screen capture | GDI (misses fullscreen DX games) | DXGI Desktop Duplication (catches everything) |
| Color averaging | Per-channel mean | Gamma-correct linear-light |
| Installer size | ~250 MB | ~10 MB |
| RAM idle | ~170 MB | ~50 MB |
| Subscription | $20/yr | Free |
| Web/phone control | Not built-in | Built-in, opt-in (bearer token) |

## How it differs from Govee Home

| | Govee Home | LightSync |
|---|---|---|
| RGBIC segment effects | Yes (proprietary dreamview) | No — LAN API exposes one color per device |
| Cloud / firmware updates | Yes | No (by design) |
| LAN control | Optional toggle | Always-on |
| Screen sync | Basic | Gamma-correct, 30/60 fps, multi-monitor (Phase 2) |
| Automation / scripting | No | HTTP API + bearer token (Phase 5) |

## Security

Govee's LAN protocol is unauthenticated UDP multicast. **Anyone on your Wi-Fi can address your lights.** That's Govee's design, not ours — we don't worsen it, but we don't pretend it's safer. We do:

- Document the limitation in the UI ("Anyone on your Wi-Fi can address your lights").
- Recommend router isolation for guest networks.
- Add a bearer-token-authenticated HTTP control server, off by default, for phone access.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│ LightSync.exe (Tauri 2, single binary, ~10 MB)      │
│                                                     │
│ ┌──────────────────┐  ┌──────────────────────────┐  │
│ │ Rust core (tokio)│  │ React + TS UI (WebView2) │  │
│ │                  │  │                          │  │
│ │ Govee LAN client │  │ Device list, color picker│  │
│ │ DXGI capture     │◄─┤ Effects, settings        │  │
│ │ Effect engine    │  │                          │  │
│ │ axum HTTP (off)  │  │                          │  │
│ └──────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────┘
          │                                  ▲
          │ UDP multicast 239.255.255.250    │ DXGI (Windows)
          ▼                                  │
   ┌────────────────┐               ┌────────┴────────┐
   │ Govee devices  │               │ Your screen     │
   └────────────────┘               └─────────────────┘
```

## Project layout

```
lightsync/
├── src/                    # React + TypeScript frontend
│   ├── App.tsx
│   ├── components/         # DeviceList, EffectPicker, Settings
│   └── hooks/              # useDevices, useEffectStore
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── govee/          # LAN protocol, discovery, device control
│   │   ├── capture/        # DXGI + color sampling (Windows)
│   │   ├── effects/        # solid, breathing, rainbow, screen_sync
│   │   ├── http/           # axum bearer-token server
│   │   ├── state.rs        # AppState
│   │   └── lib.rs          # Tauri entry, command surface
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/workflows/
│   └── build.yml           # Windows MSI/NSIS CI
├── package.json
└── README.md
```

## License

MIT. See [LICENSE](./LICENSE).

## Acknowledgements

- Govee LAN protocol shape from Govee's public developer guide.
- DXGI capture reference: Microsoft Learn Desktop Duplication docs, DXcam.