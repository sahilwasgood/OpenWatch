# OpenWatch — research summary (2026-11-22)

Eight parallel research lanes. Findings that changed the design are recorded here with sources.

## 1. GUI framework — Slint first, Tauri 2 second
- Slint 1.19 has official Android (minSdk 26, Skia) and `backdrop-blur` on translucent Rectangles;
  native renderer, no WebView, small footprint, declarative `.slint` DSL. https://slint.dev/blog/slint-1.19-released.html
- Tauri 2 is strongest for glass (`backdrop-filter`, `window-vibrancy` `apply_liquid_glass`) but WebView
  look varies by OS and default templates look generic. https://v2.tauri.app/
- egui is light and popular but "utility" looking; glass needs registered backdrop/2-pass work.
- iced has no shipped Android; GTK4/Leptos are not cross-platform answers here.
- Decision: Slint, with Tauri 2 as fallback if web tooling becomes decisive.

## 2. BLE — btleplug + Tokio
- btleplug 0.13.x is the only library covering Win/macOS/Linux/Android from Rust with discovery,
  read, write, subscribe, notification streams. https://github.com/deviceplug/btleplug
- `bluest` lists Android as planned; `blew` has no Windows; SimpleBLE is commercially licensed.
- BlueZ quirks: merged scan filters, BR/EDR vs LE `AddressType` trap (ControllerMode=le is the fix).
- macOS: CoreBluetooth exposes a per-host UUID, never the MAC → scan-and-match by advertised name.
- Android: API 24+, runtime scan/connect permissions, R8 keep rules.
- Decision: shared `BleTransport` returning a `Stream`; `DeviceKey` resolver for Apple.

## 3. Prior art
- Gadgetbridge (AGPL, Codeberg): per-device coordinator + capability matrix, local-only, F-Droid,
  Weblate. https://codeberg.org/Freeyourgadget/Gadgetbridge
- jphein/moyoung-watch (AGPL): clean-room FE EA protocol, offline pytest fixtures, protocol doc.
- neubofy/Watch (GPL): Kotlin/Compose native Moyoung protocol, Health Connect, zero-touch reconnect.
- pinepal (GPL): Rust GTK4 InfiniTime companion with exponential-backoff reconnect + background service.
- Clepsydre (MIT/Apache, Codeberg): notification blacklist/prefix trimming for small screens.
- Fogg, DaFit Canvas: visual face designers; Fogg's license is unstated — do not reuse code.
- Port: coordinator model, resilient connection, offline fixtures, standalone face designer.
- Avoid: CSV-in-preferences health storage, vendor cloud/account dependencies, unlicensed dumps.

## 4. Liquid glass UI spec
- Glass for navigation/chrome/cards only; content and long text on solid layers.
- Regular card `rgba(255,255,255,.22)` + `blur(20px) saturate(140%)`; dark `rgba(28,28,30,.6)`;
  1 px light edge; 14/24/28–36 px concentric radii; shadow `0 22px 70px rgba(0,0,0,.35)`.
- Zinc ramp + one blue accent; status green/amber/red; color on backgrounds, not labels.
- Type 11/13/15/17/22/34 pt, tabular numerals; 4 pt spacing base; rows 52–56 px.
- Motion 250 ms `cubic-bezier(.2,.8,.2,1)`, press 120 ms; reduced-motion → 150 ms cross-fade.
- Reduced-transparency → solid surfaces; high contrast/forced colors fallbacks; 4.5:1 body contrast.
- Sources: Apple HIG Materials, WWDC25 "Meet Liquid Glass", NN/g and Infinum accessibility critiques.

## 5. Packaging and CI
- Cargo workspace: pure `core` + thin desktop/android frontends.
- Tauri Android / cargo-mobile2 for APK/AAB; cargo-ndk for Gradle integration.
- Desktop artifacts: AppImage/deb/rpm, MSI/NSIS, dmg; notarization needs a paid Apple account.
- GitHub Actions matrix Ubuntu/macOS/Windows + arm64 on public repos; `tauri-action@v1` for releases.
- Public repo enables free Actions minutes, secret scanning, push protection, Dependabot.
- Decision: public GPL-3.0-or-later repo; CI matrix on tag `app-v*`.

## 6. Feature inventory (local toolchain audit)
- 20 live-verified reads, framing/GATT details, all writes and confidence tags are in the design doc.
- Important gap: legacy console had camera `0x66`, ring `0x62`, move reminders `0x83`, women's
  health `0x85`, message-app list `B9{20,0}`, stress chart, continuous-HR list, diagnostics,
  OTA/AGPS — all dropped from the current Python set and even the current HTML console. Recover
  from ledger/source notes and tag unverified.

## 7. User reviews / complaints
- Top complaints: silent disconnects and no auto-reconnect, forced accounts/login walls, paywalled
  faces, wrong units and implausible health numbers, missing notification reader, deep menus.
- Most wanted: local-first/no internet permission, editable faces with multiple simultaneous fields,
  visible sync states, reorderable at-a-glance cards, export/integrations, background sync.
- Rust GUI reality check: mature choices are egui/Tauri; budget for accessibility, IME, WebView
  inconsistency and compile friction.
- Sources: r/smartwatch cheap-watch thread, GrapheneOS health-tracker thread, Da Fit/NoiseFit store
  reviews, Garmin Connect complaints, weeklyrust GUI survey.

## 8. Closed-source UX references
- Recurring good pattern: persistent connection strip (battery, last sync, Sync now), visible sync
  states, reorderable metric cards, one notification panel, device page owning faces + firmware.
- Avoid: paywalls, nested update paths, parity that varies by host OS, hidden familiar metrics.
- References: Fitbit, Garmin Connect, Samsung Wearable, Huawei Health, Zepp Life, Apple Watch,
  Da Fit, NoiseFit, Fire-Boltt Fit (vendor docs/store pages/redesign coverage).
