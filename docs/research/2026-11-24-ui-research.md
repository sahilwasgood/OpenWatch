# UI/UX research behind the OpenWatch redesign (2026-11-24)

Six parallel research lanes. The decisions they drove are in
`docs/superpowers/specs/2026-11-23-ui-redesign-design.md`.

## Modern app UI patterns
- One accent, achromatic chrome. Linear: near-black gray ramp + one violet reserved for CTAs/active states.
  Raycast: hairline borders, one white CTA, zero ornamental shadow. Vercel: blue for links only.
- Density is a feature for power users, but only with generous space around the dense block.
- Semantic type scale (Apple named styles, Material 3 15 styles), tabular numerals for metrics.
- Elevation on dark = luminance steps (white opacity .02→.05) + hairlines, not heavy shadows.
- Designed empty/loading/error states; motion discipline (exit faster than enter); visible focus.
- Numbers: 44pt min tap target, 48pt comfortable; spacing 4/8/16/24/32; radii 6/8/12;
  motion 200/300/400/500ms; contrast 4.5:1 body, 3:1 large/UI, 7:1 AAA target.
- Sources: Apple HIG typography, m3.material.io type/motion, Linear redesign writeups, shadcn theming,
  vercel.com/geist/typography, stripe.design, 1Password 8 design blog.

## Slint 1.19 capabilities (installed sources + docs)
- `backdrop-blur` and `inner-shadow-*` are Skia-only; solid fallbacks mandatory.
- `drop-shadow-*`, gradients (`@linear-gradient`, `@radial-gradient`), `border-radius`, `opacity`, `clip` work.
- No FlexLayout in 1.19 — compose HorizontalLayout/VerticalLayout/GridLayout with stretch.
- `for`/`if` repeaters, ListView; Text with font-family/size/weight/letter-spacing/overflow/wrap;
  `StyledText` for CommonMark subset; fonts registered in Rust before backend init.
- SVG via `@image-url` + `colorize` (resvg `svg` feature) — ideal for a themed icon family. `Path` also available.
- `animate` with `ease`/`cubic-bezier`/`spring` and `states` blocks. Reduced-motion is applied automatically
  by the compiler from platform context, not author-controlled in `.slint`.
- `FocusScope` + `accessible-role/label/checked/expanded/...` for keyboard and AT.
- NOT possible: blur/inner-shadow on non-Skia renderers, CSS filter/blend modes, FlexLayout,
  `@font-face`/runtime web fonts, author-controlled reduced-motion.

## Companion-app UX
- First screen = device state (battery, connection, last sync, firmware), not a wall of metrics.
- Two-level navigation max, permanent Home escape; Garmin/Fitbit/HA complaints centre on deep menus.
- Distinct states: paired / connected / syncing / stale / disconnected / failed, with last-sync time.
- Sync as first-class progress state; charts few/large/user-orderable with raw values on tap.
- Settings grouped by user goal, not protocol; destructive actions name the exact consequence.
- Notifications default off with per-app opt-in; faces browse→preview→install→manage.
- Firmware and raw protocol data are power-user surfaces, quarantined from main nav.
- Sources: Gadgetbridge docs, and user threads for Garmin Connect, Fitbit, Home Assistant, Tailscale,
  Apple Watch, FossilHybrids, Tesla.

## Visual craft spec (dense technical dashboard)
- Zinc ramp `950 #09090b … 50 #fafafa`; text primary #fafafa (19:1 on zinc-950), secondary #a1a1aa (6.9:1).
- One accent #2563eb (white 5.17:1); NEVER #3b82f6 behind white (3.68:1). Our app chose #0a84ff with #09090b label.
- Surfaces: app #09090b, panel #18181b, raised #27272a, selected #3f3f46; borders #27272a/#3f3f46.
- Radii 4/6/8/12; spacing 0/4/8/12/16/24/32/48/64; type 24/18/15/13/12/11 with tabular numerals.
- Icons Lucide-style 16px grid, 1.75px stroke; ≤5 chart series, gridlines #27272a.
- Glass: rgba(24,24,27,.72) + blur(12px), borders rgba(255,255,255,.08); compute contrast after compositing.
- Motion: hover 120ms, press 90ms, state 180ms, entrance 220ms, modal 240ms; standard cubic-bezier(.2,0,0,1).
- Sources: Tailwind palette, Radix color scale docs, WCAG 2.2 SC 1.4.3/1.4.11, Material 3 motion.

## Accessibility / anti-slop
- WCAG 2.2: 4.5:1 text, 3:1 large text and UI components; measure composited glass at every gradient stop.
- Focus ring ≥2px with 3:1 against adjacent pixels; 24×24px minimum target (44dp comfortable);
  status triple-encoded (color + icon + text); 200% text scaling; reduced-motion and reduced-transparency.
- Android has no reduced-transparency system preference — in-app toggle required.
- AI-slop tells → alternatives: purple gradient→one accent; nested rounded cards→space+headings;
  border/CTA on every block→reserve for state; emoji icons→one coherent icon family;
  identical fade-up→one easing family; fake metrics→real task data; "NEW/LIVE" badges→only when actionable.
- Sources: W3C WCAG 2.2 understanding docs, MDN, Android Compose semantics/animation guidance,
  practitioner writeups on slop UI.

## Current UI audit (pre-redesign)
- P0: every write control was inert; accent fills used light text ~3.3:1; sync failures discarded;
  fixed 1040×720 geometry; empty Workbench rendered placeholder garbage; settings toggles unreachable;
  units toggle cosmetic; chart zero-based peak normalization misleading.
- Good and preserved: tokens and solid fallbacks, UTC stamps, hex confined to Workbench, L/S/U tags,
  two-step confirmations, no-key-zero-request, byte-identical preview, Flickables and elide on Device rows.
