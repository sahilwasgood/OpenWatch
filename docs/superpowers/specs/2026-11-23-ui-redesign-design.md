# OpenWatch UI redesign — design document

Date: 2026-11-23. Status: proposed. Product: OpenWatch (Rust + Slint 1.19).

## Intent and why
The current UI compiles and has a coherent token system, but it is a fixed 1040×720 desktop shell with
inert write controls, weak empty/error states, and several contrast/target-size failures. The operator
asked for research-led UI/UX work that makes the app genuinely beautiful and modern without "AI slop".
This design turns the six research briefs into concrete, testable changes while keeping the native
Rust/Slint stack.

## Goals
- Make every advertised control real: Today's "Control" affordance must open a sheet wired to `Action`.
- Distinct, honest sync states: paired / connected / syncing / stale / disconnected / failed with last-sync time.
- Layout that adapts from a compact desktop window to Android instead of absolute 1040×720 coordinates.
- One restrained visual language: neutral zinc ramp, one accent, deliberate density, designed empty/loading/error states.
- Accessibility: ≥4.5:1 text, ≥3:1 UI boundaries, ≥40px touch targets, ≥12px type, reachable toggles, labels.
- Preserve what already works: UTC stamps, hex confined to Workbench, L/S/U tags, two-step confirmations,
  zero-network-without-key, byte-identical AI preview.

## Non-goals / anti-goals
- No framework change: no Electron, React Native, or webview. Slint 1.19 stays.
- No nested glass cards, purple gradients, glow, emoji icons, hero sections, fake stats, or bounce easing.
- No new cloud dependency; local-first stays.
- Do not move raw protocol data onto Today or Device.

## Constraints
- Slint 1.19 has no FlexLayout, no CSS filter/blend modes, no runtime web fonts, no Skia blur on non-Skia renderers.
- Backdrop-blur and inner-shadow are Skia-only; solid fallbacks are mandatory.
- Reduced-motion is platform-driven; Android has no reduced-transparency OS preference — in-app toggle required.
- Generated Slint types cannot cross crate boundaries; all UI stays in `crates/ui/`.

## Decisions (builder's calls)
1. **Accent stays `#0a84ff`** with near-black `#09090b` text on accent fills (≈5.45:1). Swapping to `#2563eb`
   would churn every test and token for no user-visible gain. Applied consistently, not only in the nav.
2. **Glass is restrained to the connection strip and device card**; all other surfaces are solid luminance steps
   with 1px hairlines. This honours the earlier "liquid glass" request without the nested-glass slop tell.
3. **Type scale** (px): display 24/650/32, H1 18/600/24, H2 15/600/20, body 13/400/20, label 12/500/16,
   micro 11/500/14. Minimum interactive label 12px.
4. **Spacing** 4/8/12/16/24/32; radii 6 (button/input), 8 (card), 12 (panel/popover); dividers 1px.
5. **Motion**: hover 120ms, press 90ms, state 180ms, panel 220ms, modal 240ms; one easing family
   `cubic-bezier(.2,0,0,1)`; exit faster than enter; animate geometry only via opacity/position.
6. **Elevation** from zinc steps: app `#09090b`, panel `#18181b`, raised `#27272a`, selected `#3f3f46`;
   borders `#27272a` / `#3f3f46`; no heavy shadows.
7. **Icons**: one small SVG set via `@image-url` + `colorize` bound to theme tokens; no emoji.
8. **Navigation stays four destinations** (Today, Device, Workbench, Data) with a common pane header.
   Advanced/mod actions stay inside Workbench, never on Today/Device.

## Parts of the how left to the implementer
- Exact Slint layout expressions that achieve responsiveness without breaking the existing geometry tests.
- The control-sheet component shape and how `Action` values map to rows.
- The specific SVG icon assets and their paths.
- Test structure for each change; keep or update existing assertions in `crates/ui/src/lib.rs`.

## Success criteria
- A mocked write test proves a control in the UI triggers `act()` and the exact frame is written.
- No accent-filled surface uses light text; contrast assertions cover the new cases.
- The window renders acceptably at a smaller desktop size and on a narrow Android viewport (layout-based checks).
- Workbench and Data have real empty, loading, error, and success states.
- All workspace tests pass; fmt/clippy clean.

## Builder-check decisions (operator delegated; recorded as builder's calls)
1. **Responsive targets:** minimum desktop window 900×600; minimum Android viewport 360dp. Wide (≥700px) keeps
   the 220px sidebar; narrow (<700px) switches the four destinations to a bottom navigation bar. Panes use
   layouts and proportional widths, not absolute coordinates. Cost if wrong: another layout pass.
2. **Sync state authority:** core `ConnectionState` remains authoritative for connection; the UI keeps its own
   `syncing` flag and a `failed` flag from the last refresh result. "Stale" means no successful refresh in
   5 minutes. No cross-crate state-machine change in this iteration. Cost if wrong: labels may need remapping.
3. **Control sheet:** two-step confirm for every state-changing write. Only cards with a safe write mapping
   (goal, time system, DND, auto-HR, alarm, time sync, message, live HR — all L-verified) show a Control
   affordance. S/U actions stay preview-only in Workbench. Cost if wrong: a card may lack a control.
4. **Threading scope:** the blocking refresh and synchronous AI POST are NOT part of this UI redesign; their
   loading states are driven by existing flags/status strings and the freeze remains an open backlog item.
   Cost if wrong: a real progress bar cannot animate during the blocked call.
