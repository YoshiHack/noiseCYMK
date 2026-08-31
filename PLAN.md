# LightSync — SignalRGB Replacement for Govee (Windows, Self-Hosted)

> **Working title only — name TBD.** This is a from-scratch RGB controller designed around the gaps in SignalRGB's Govee integration.

**Goal:** A lightweight Windows desktop app that controls Govee Wi-Fi lights (and any other RGB peripherals you add later) over the local network — no cloud dependency, no subscription, no conflicts with the Govee Home app — with screen-color sampling for ambient lighting.

**Architecture:** Local-only Windows desktop app (Tauri 2 + Rust backend + React frontend) that talks directly to Govee devices via their **LAN API** (UDP multicast discovery on `239.255.255.250`, control commands on UDP/4003). Screen capture uses Windows DXGI Desktop Duplication API in a Rust thread for sub-frame latency. A future web-control layer (Tauri's local HTTP server, LAN-only) is built in from day one but disabled by default.

**Tech Stack:**
- **Shell:** Tauri 2 (Rust + WebView2) — ~10 MB installer, ~50 MB RAM, smaller than Electron, single binary, ships with a built-in local HTTP server
- **Backend:** Rust — `tokio` for async, `socket2` for multicast/UDP, `serde_json` for Govee protocol, `windows` crate for DXGI capture
- **Frontend:** React + TypeScript + Tailwind (Vite) — your existing stack-adjacent, easy to skin
- **Distribution:** Tauri MSI/NSIS installer for Windows x64
- **Future:** Optional LAN HTTP server (`axum`) reusing Tauri's IPC for browser control

---

## Critical discovery: what "OTA" actually means in your case

You said *SignalRGB won't work with my Govee devices OTA*. Two possibilities — make sure I'm solving the right one:

1. **Most likely:** You meant **over-the-LAN / over-Wi-Fi** (i.e., Wi-Fi-based lights, not USB peripherals). SignalRGB's Govee integration is *exactly* this — LAN control on `239.255.255.250:4003` — but it has well-documented reliability problems: LAN toggle gets hidden behind firmware updates, the Govee Home app and SignalRGB fight for control of the same multicast socket on UDP 4002 (only one app can bind it at a time), and many RGBIC strips are detected as a single zone because the LAN API only exposes one color at a time. This is the whole reason you're frustrated.
2. **Less likely but worth checking:** If by "OTA" you literally meant **over-the-air firmware updates** (Govee pushing new firmware to your devices), that path runs through Govee's cloud and would require the Govee Developer API key plus the device's `sku`. SignalRGB doesn't expose this — it's read-only device control, not firmware management. If this is actually your need, say so and I'll add a small "Govee Cloud" module for firmware check + trigger, but it's a separate code path.

**Confirmed by user: interpretation #1 (LAN).** Screen-color sampling is in scope regardless.

## Your Govee devices (identified from app screenshots you sent)

| SKU | Type | Notes for capability map |
|---|---|---|
| **H6046** | RGBIC LED light bars (pair) | Single-zone LAN; RGBIC segment data not exposed by LAN API |
| **H610A** | RGBIC LED strip | Single-zone LAN; same limitation |
| **H6609** | TV Backlight 3 Lite (camera + RGBIC strip) | Camera-based ambient; the strip itself is single-zone LAN. The dreamview camera flow is proprietary and out of scope. |

**Capability detection:** I'll add a per-SKU table in `govee/capabilities.rs` keyed on these three SKUs. Any device we discover whose SKU isn't in the table falls back to "RGB+CCT single-zone" — the conservative LAN default. Discovery will still succeed; capabilities may just be limited.

---

## Architecture diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  LightSync.exe  (Tauri 2, single binary, ~10 MB)                │
│                                                                 │
│  ┌───────────────────────┐    ┌──────────────────────────┐      │
│  │  Rust core (tokio)    │    │  React + TS UI           │      │
│  │                       │    │  (WebView2)              │      │
│  │  ┌─────────────────┐  │    │                          │      │
│  │  │ Govee LAN client│  │    │  - device list           │      │
│  │  │  - discovery    │◄─┼────┤  - color picker          │      │
│  │  │  - color/bright │  │ IPC│  - effect picker         │      │
│  │  │  - state query  │  │    │  - screen-capture toggle │      │
│  │  └─────────────────┘  │    │  - settings              │      │
│  │  ┌─────────────────┐  │    │                          │      │
│  │  │ Screen sampler  │  │    │                          │      │
│  │  │  - DXGI capture │  │    │                          │      │
│  │  │  - per-zone     │  │    │                          │      │
│  │  │    color avg    │  │    │                          │      │
│  │  │  - 30-60 fps    │  │    │                          │      │
│  │  └─────────────────┘  │    │                          │      │
│  │  ┌─────────────────┐  │    │                          │      │
│  │  │ Effect engine   │  │    │                          │      │
│  │  │  - solid        │  │    │                          │      │
│  │  │  - breathing    │  │    │                          │      │
│  │  │  - rainbow      │  │    │                          │      │
│  │  │  - screen-sync  │  │    │                          │      │
│  │  └─────────────────┘  │    │                          │      │
│  │  ┌─────────────────┐  │    │                          │      │
│  │  │ LAN HTTP server │  │    │                          │      │
│  │  │  - opt-in       │  │    │                          │      │
│  │  │  - port 7878    │  │    │                          │      │
│  │  │  - bearer token │  │    │                          │      │
│  │  └─────────────────┘  │    └──────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
            │                                      ▲
            │ UDP multicast                        │ DXGI
            ▼                                      │ (Windows)
   ┌────────────────┐                       ┌──────┴──────┐
   │ Govee devices  │                       │ Your screen │
   │ 239.255.255.250│                       └─────────────┘
   │ :4001 scan     │
   │ :4003 cmd      │
   └────────────────┘
```

---

## Why this stack

| Choice | Rationale |
|---|---|
| **Tauri 2, not Electron** | Electron idles at ~170 MB RAM for a 1-window app; Tauri idles at ~40 MB. Screen capture at 60 fps is GPU-bound; we don't need a second Chromium instance eating cycles. Rust backend is the natural place for UDP + DXGI. |
| **DXGI Desktop Duplication, not GDI/MSS** | GDI captures (`mss`, `PIL.ImageGrab`) can't capture fullscreen DX11/12 apps (games). DXGI can. Rust library `windows` crate exposes this directly; `DXcam` proves the pattern. 60+ fps even on 144 Hz panels. |
| **Tokio UDP, not std::net in a sync thread** | Discovery and control need concurrent sockets on 4001/4002/4003. Tokio's the standard. |
| **No cloud, no telemetry** | Your data stays on your machine. Reads Govee's documented unauthenticated LAN protocol — no API key, no rate limits. |
| **Web control from day one (but off)** | Tauri gives you a localhost IPC. Slapping `axum` on top for `http://<lan-ip>:7878/api/...` with a bearer token is ~100 lines. Disabled by default. When you want it, you toggle a switch in settings. |

---

## Project layout

```
lightsync/
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs             # Tauri entry, command routing
│   │   ├── govee/              # Govee LAN module
│   │   │   ├── mod.rs          # Device registry
│   │   │   ├── discovery.rs    # UDP multicast scan
│   │   │   ├── device.rs       # Single-device control
│   │   │   ├── protocol.rs     # Govee JSON message types
│   │   │   └── cloud.rs        # (Optional) Govee cloud API for OTA
│   │   ├── capture/            # Screen color sampling
│   │   │   ├── mod.rs
│   │   │   ├── dxgi.rs         # DXGI Desktop Duplication wrapper
│   │   │   └── sampler.rs      # Per-zone color averaging
│   │   ├── effects/            # Effect engine
│   │   │   ├── mod.rs
│   │   │   ├── solid.rs
│   │   │   ├── breathing.rs
│   │   │   ├── rainbow.rs
│   │   │   └── screen_sync.rs  # Pipes capture → devices
│   │   ├── http/               # Optional LAN control server
│   │   │   ├── mod.rs
│   │   │   └── auth.rs         # Bearer-token middleware
│   │   └── state.rs            # AppState (device map, settings)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # React frontend
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── DeviceList.tsx
│   │   ├── ColorPicker.tsx
│   │   ├── EffectPicker.tsx
│   │   ├── ScreenCapturePanel.tsx
│   │   └── Settings.tsx
│   ├── hooks/
│   │   └── useTauri.ts         # Wraps invoke() + listen()
│   └── styles/
├── tests/
│   ├── govee_protocol_tests.rs # JSON round-trip, command shape
│   ├── sampler_tests.rs        # Color averaging (deterministic fixtures)
│   └── integration_smoke.rs    # Spin up backend, mock UDP, exercise IPC
├── package.json
├── vite.config.ts
└── README.md
```

---

## Step-by-step build plan

Each task is bite-sized (2–5 min of focused work) and ends with a commit. TDD throughout where the logic is testable.

### Phase 0 — Bootstrap (1 session)

**Task 0.1:** Initialize Tauri 2 project with React + TS + Tailwind template. Verify `cargo tauri dev` opens an empty window. Commit.

**Task 0.2:** Set up `AppState` struct (device map, settings, control channels). Wire `tauri::Builder` to share state via `manage()`. Commit.

**Task 0.3:** Add `Cargo.toml` deps: `tokio`, `serde`, `serde_json`, `socket2`, `windows`, `axum`, `parking_lot`, `anyhow`. Add npm deps: `zustand` (state), `react-colorful` (picker). Verify cargo check + tsc pass. Commit.

### Phase 1 — Govee LAN protocol (the actual SignalRGB-replacement core)

**Task 1.1:** Implement `protocol.rs` — serde structs for `RequestMessage`, `ResponseMessage`, `ScanRequest`, `ScanResponse`, `DevStatusRequest`, `TurnCommand`, `BrightnessCommand`, `ColorRgbCommand`. **TDD:** write round-trip JSON tests against fixtures copied from the official Govee LAN guide. Run `cargo test`, expect pass. Commit.

**Task 1.2:** Implement `discovery.rs` — bind UDP socket to `0.0.0.0:4002` (with `SO_REUSEADDR` so we coexist with govee2mqtt if needed — but on Windows you're mostly alone here), join multicast group `239.255.255.250`, send `ScanRequest` to `239.255.255.250:4001`, listen on 4002 for `ScanResponse` packets, populate a `HashMap<DeviceId, LanDevice>` with TTL of 60 s. **TDD:** mock socket with a fake receiver that returns canned JSON; assert devices populate. Commit.

**Task 1.3:** Implement `device.rs` — `set_color(rgb: u8, u8, u8)`, `set_brightness(pct: u8)`, `set_power(on: bool)`, `query_state()`. Each serializes the right JSON and sends UDP to `device_ip:4003`. **TDD:** assert outgoing bytes match the Govee spec exactly. Commit.

**Task 1.4:** Wire Govee commands into Tauri's IPC. Add `#[tauri::command] async fn list_devices()`, `set_device_color(id, r, g, b)`, etc. Frontend gets a `useDevices()` hook. Smoke test: discover on a real LAN, change a light. Commit.

**Task 1.5:** Handle the **UDP 4002 conflict** that plagues SignalRGB. Add a setting: "Allow Govee Home app to also run" — if on, we use broadcast + unicast scanning without binding 4002 (slower but doesn't fight the official app). Document the tradeoff in Settings. Commit.

**Task 1.6 (optional, only if you confirm O above):** Add `cloud.rs` — Govee HTTP API with your API key, for `devices/firmware` check + `devices/firmware/update` trigger. Skip until you've confirmed. Commit if added.

### Phase 2 — Screen color capture (the ambient-lighting side)

**Task 2.1:** Implement `dxgi.rs` — wrap `IDXGIFactory1` → `IDXGIAdapter1` → `IDXGIOutput1` → `IDXGIOutputDuplication`. `grab_frame()` returns a BGRA `Vec<u8>` + dirty-rect metadata. **TDD:** device enumeration in a test; capture skipped in CI (no display). Commit.

**Task 2.2:** Implement `sampler.rs` — given a frame + a `Vec<ScreenZone>` (rectangles), compute average color per zone in linear-light RGB (gamma-correct, not naive mean — that's the #1 ambient-lighting bug). **TDD:** feed a solid-red 100×100 fixture, assert average is `(255, 0, 0)`. Feed a checkerboard, assert it's gray. Commit.

**Task 2.3:** Add screen-capture settings: zones (named rectangles like "left", "top", "bottom"), frame rate (30/60), saturation/gamma multipliers, smoothing factor (EMA between frames to kill flicker). Persist to `settings.json` in `%APPDATA%/LightSync/`. Commit.

**Task 2.4:** Add `screen_sync.rs` effect — captures at chosen FPS, samples configured zones, maps each zone → one device (or one segment on a multi-zone device), pushes colors through the device control channel. Commit.

**Task 2.5:** Frontend: `ScreenCapturePanel` with a preview showing the capture rectangles overlaid on a thumbnail of the user's current screen (one-shot capture for preview, not live — saves bandwidth to WebView). Commit.

**Task 2.6:** Multimonitor: enumerate all `IDXGIOutput`s, let user assign each monitor's edges to specific lights. Commit.

### Phase 3 — Effects engine

**Task 3.1:** `solid.rs` — single color to all selected devices. Trivial; just a passthrough. **TDD:** verify color is sent each frame at the chosen FPS. Commit.

**Task 3.2:** `breathing.rs` — sine-wave brightness modulation around a base color. **TDD:** assert brightness values follow `sin(t)` over a fixed clock. Commit.

**Task 3.3:** `rainbow.rs` — HSV hue rotation across devices and over time. **TDD:** assert color sent at frame N matches `hsv(N % 360, 1, 1)`. Commit.

**Task 3.4:** Effect scheduler — single tokio task drives whichever effect is active; broadcasts state changes (pause when device disconnects, resume when it returns). Commit.

### Phase 4 — UI polish

**Task 4.1:** `DeviceList` — shows online/offline badges, per-device color picker, on/off toggle, brightness slider. Drag-to-reorder for multi-device setups. Commit.

**Task 4.2:** `EffectPicker` — radio cards with live previews (mini canvas). Commit.

**Task 4.3:** `Settings` — startup-with-Windows toggle, minimize-to-tray, conflict-mode for Govee Home, LAN HTTP server toggle (default OFF), bearer-token display + regenerate. Commit.

**Task 4.4:** System tray icon with right-click menu (open window, quit, current effect name). Single-instance lock so launching twice focuses the existing window. Commit.

**Task 4.5:** Dark theme + system theme follow. Match your existing brand-neutral aesthetic (no bright neon — let the lights be the lights). Commit.

### Phase 5 — LAN HTTP control (off by default)

**Task 5.1:** `axum` server bound to `0.0.0.0:7878` only if enabled in settings. Endpoints:
- `GET /api/devices`
- `POST /api/devices/{id}/color` body `{ "r": u8, "g": u8, "b": u8 }`
- `POST /api/devices/{id}/power` body `{ "on": bool }`
- `POST /api/effect` body `{ "effect": "solid", "color": [...] }`
- Bearer token middleware. Token generated on first launch, shown in Settings.
Commit.

**Task 5.2:** Tiny web UI served at `/` — single HTML page that just calls the API. Lets you control lights from your phone on the same Wi-Fi. Commit.

**Task 5.3:** Documentation: security warning about the unauthenticated LAN protocol (anyone on your Wi-Fi can control the lights — this is fundamental to Govee, not a bug in our app). Recommend router isolation for guest networks. Commit.

### Phase 6 — Ship

**Task 6.1:** GitHub Actions workflow — `cargo test` + `cargo tauri build` on Windows runner, produces signed (or unsigned, your call) MSI artifact. Commit.

**Task 6.2:** README with quickstart, screenshots, "How it differs from SignalRGB", known-supported Govee SKUs (the ones on Govee's compatibility list), and a section on adding new device types. Commit.

**Task 6.3:** First release tag `v0.1.0`. Commit.

---

## What I will NOT do (out of scope, by design)

- **USB peripheral support** (motherboard RGB, Corsair iCUE, Razer Chroma). OpenRGB already does this and it's a *huge* surface. If you want it later, the right move is to embed the OpenRGB SDK server and translate — but that's Phase 7, not Phase 0.
- **Cloud control of Govee.** Cloud is the source of every problem you hit. Local LAN only.
- **Mobile apps.** The LAN HTTP server + the web UI at `/` is your phone interface. No native mobile builds.
- **Cross-platform.** Windows only as requested. Tauri's cross-platform is a Phase-7 thing if you ever want it.
- **OTA firmware pushing** unless you confirm you meant that (see "Critical discovery" above).

---

## Risks and tradeoffs

| Risk | Mitigation |
|---|---|
| Govee LAN toggle hidden behind firmware updates | Document the Govee Home app path to enable LAN; surface a check in our settings that says "if no devices found, verify LAN Control is on in Govee Home." |
| UDP 4002 conflict with Govee Home | Conflict-mode option in Settings (Phase 1, Task 1.5) |
| RGBIC multi-segment strips only show one color over LAN | Document the limitation honestly; this is Govee's LAN API, not our bug. RGBIC streaming needs the proprietary Dreamview protocol which Govee hasn't published. |
| DXGI capture fails on hybrid GPUs / WSL / VMs | Detect adapter on startup, show clear error in UI, fall back to GDI capture if DXGI fails (with a "lower quality" warning). |
| LAN control is unauthenticated UDP | Security warning in docs; opt-in LAN HTTP server; recommend router isolation. We don't *worsen* this — the Govee LAN protocol is already this way — but we also don't pretend it's safer than it is. |
| Tauri 2 ecosystem still maturing | Pinned versions, tested in CI on a clean Windows runner. If Tauri breaks, we ship a CLI fallback so the Rust core can still be used headlessly. |

---

## What I'll need from you to start

1. **Confirm the OTA interpretation.** Did you mean LAN (Wi-Fi control) or actual over-the-air firmware updates? (See "Critical discovery" above.) Plan changes meaningfully between the two.
2. **List your Govee devices** — exact model numbers (H6163, H619Z, H61A0, etc.). This lets me pull the right SKU capability list up front and validate discovery in Phase 1.
3. **Windows version** — 10 or 11? DXGI behavior is mostly the same but DirectX 12 capture requires Win10 1903+.
4. **Are you okay with Tauri 2 + Rust?** It's the right technical choice for a low-RAM, fast screen-capture app, but it does mean a Rust toolchain on your Windows box (Visual Studio Build Tools + Rust + WebView2 — Tauri has a one-line installer that does all of this).
5. **Build it on Windows directly, or build it here and ship the installer?** I'm running on Ubuntu so I can't *run* the Windows binary, but I can author the whole project and hand you a build script + CI that produces installers for your machine. If you want, we can also set up GitHub Actions so every push produces a downloadable MSI without you building locally.
6. **Name.** I called it "LightSync" in the plan as a placeholder. You tell me what it actually is.

---

## Files to touch (concrete)

- `src-tauri/src/govee/protocol.rs` — JSON message types (new)
- `src-tauri/src/govee/discovery.rs` — UDP multicast scan (new)
- `src-tauri/src/govee/device.rs` — device control commands (new)
- `src-tauri/src/govee/cloud.rs` — optional firmware OTA (new, conditional)
- `src-tauri/src/capture/dxgi.rs` — DXGI capture (new)
- `src-tauri/src/capture/sampler.rs` — color averaging (new)
- `src-tauri/src/effects/*.rs` — effect engine (new)
- `src-tauri/src/http/mod.rs` — LAN control server (new)
- `src-tauri/src/main.rs` — Tauri entry, command routing (modify)
- `src/components/*.tsx` — React UI (new)
- `.github/workflows/build.yml` — Windows CI (new)
- `README.md`, `LICENSE`, `CHANGELOG.md` (new)

---

## Tests / validation

- **Unit:** every protocol struct has JSON round-trip tests against fixtures from the official Govee LAN guide. Color sampler tested on solid, checkerboard, and gradient fixtures. Effect timing tested against a fake clock.
- **Integration:** spin up the Rust core in a test, mock the UDP sockets with canned responses, exercise the IPC commands end-to-end.
- **Manual (you, on your Windows box):** device discovery on your real LAN, color change round-trip, screen-sync effect at 30/60 fps, multi-zone mapping, conflict-mode behavior with Govee Home running, LAN HTTP server reachable from your phone.
- **CI:** GitHub Actions `windows-latest` runs `cargo test`, `npm run build`, `cargo tauri build`, uploads the MSI artifact.

---

## Saved to

`/home/jackson/projects/lightsync/PLAN.md` (project workspace created so files land there from day one).