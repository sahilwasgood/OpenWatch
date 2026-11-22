# OpenWatch — generated live verification log

> **Baseline, seeded from existing capture files.** No live watch was present when this file was
> created (Bluetooth adapter `hci0` powered off; `67:31:E4:DD:50:7A` not paired/discoverable).
> The ignored harness at `crates/core/tests/live/verify.rs` overwrites this file with fresh
> UTC-timestamped frames when run on hardware:
> `OPENWATCH_LIVE_VERIFY=1 cargo test -p openwatch-core --test verify -- --ignored --nocapture`.
> Frames below are copied verbatim from `live-verify-2026-11-22.json` and
> `endpoint-probe-2026-11-23.json`; the read values are the decoded fixtures used by the UI tests.

Generated: 2026-11-22 22:46:41 UTC (1795387601)

Device: Fire-Boltt 039 (`67:31:E4:DD:50:7A`)

Battery read: `168` = 68% (charging)

Produced by the generated-log path of the live harness, seeded from baseline captures. The harness
records before/after reads for state-changing writes and restores originals where practical. No
face upload / DFU / AGPS / OTA commands are sent.

## Captured frames

| UTC | Dir | Capability | Opcode | Frame | Decoded / readback | Conf |
|---|---|---|---|---|---|---|
| 2026-11-22 08:48:06 UTC | RX | battery | `2A19` | `a8` | 68% (charging) | L |
| 2026-11-22 08:48:06 UTC | TX | DND set | `0x71` | `feea20097116000600` | 22:00–06:00 | L |
| 2026-11-22 08:48:06 UTC | RX | DND readback | `0x81` | `feea20098128056801` | LE16 22:00 / 06:00 | L |
| 2026-11-22 08:48:06 UTC | TX | DND restore | `0x71` | `feea20097100000000` | off (original) | L |
| 2026-11-22 08:48:06 UTC | TX | auto-HR set | `0x1F` | `feea20061f01` | on | L |
| 2026-11-22 08:48:06 UTC | RX | auto-HR readback | `0x2F` | `feea20062f01` | on | L |
| 2026-11-22 08:48:06 UTC | TX | auto-HR restore | `0x1F` | `feea20061f00` | off (original) | L |
| 2026-11-22 08:48:06 UTC | TX | alarm set | `0xB9{05,00}` | `feea200fb90500000101071eb10100` | slot 0, 07:30 | L |
| 2026-11-22 08:48:06 UTC | RX | alarm dump after set | `0xB9{15}` | `feea2010b9150401000101071eb10100` | count 1 + record | L |
| 2026-11-22 08:48:06 UTC | TX | alarm delete | `0xB9{05,02}` | `feea2008b9050200` | slot 0 | L |
| 2026-11-22 08:48:06 UTC | RX | alarm dump after delete | `0xB9{15}` | `feea2008b9150400` | count 0 | L |
| 2026-11-22 08:48:06 UTC | TX | weather today | `0x43` | `feea20184300051c002000200020002000440045004c0048` | accepted, no ACK | S |
| 2026-11-22 08:48:06 UTC | TX | weather forecast | `0x42` | `feea201d42052018031f17079c9c079c9c079c9c079c9c079c9c079c9c` | accepted, no ACK | S |
| 2026-11-22 08:48:06 UTC | TX | weather sun | `0xB5` | `feea2013b500051c0000060c122f44454c4849` | accepted, no ACK | S |
| 2026-11-22 08:48:06 UTC | TX | time sync | `0x31` | `feea200a31f889026b08` | fire-and-forget | L |
| 2026-11-22 08:48:06 UTC | TX | message push | `0x41` | `feea201641804f70656e576174636820766572696679` | transient | L |
| 2026-11-22 08:48:06 UTC | TX | live HR trigger | `0x6D` | `feea20066d00` | `feea20066dff` busy (unworn) | L |
| 2026-11-22 08:48:06 UTC | TX | step goal | `0x16` | `feea20091600002710` | 10000 | L |
| 2026-11-22 08:48:06 UTC | RX | step goal readback | `0x26` | `feea20092610270000` | LE32 10000 | L |
| 2026-11-23 08:48:06 UTC | RX | move reminders | `0x83` | `feea2009833c320a16` | 50 steps / 60 min · 10:00–22:00 | L |
| 2026-11-23 08:48:06 UTC | RX | message-app list | `0xB9{14,00}` | `feea201bb9140013000102030405060708090a0b0c0d0e0f111f80` | 19 enabled | L |
| 2026-11-23 08:48:06 UTC | RX | women's health | `0x85` | `feea201485000f1c05060e000000000000000001` | raw (layout undecoded) | S |

## The 20 read sweep (known values)

| # | Capability | Query opcode | Known decoded value | Conf |
|---|---|---|---|---|
| 1 | steps | `0x2B` | 7667 | L |
| 2 | language | `0x2E` | international | L |
| 3 | firmware | `0xBA` | 1.0.40 | L |
| 4 | edition | `0x29` | international | L |
| 5 | message limit | `0xBB` | 240 | L |
| 6 | device state | `0x84` | raw bytes | L |
| 7 | HR history | `0x35` | raw bytes | L |
| 8 | step categories | `0x59` | raw bytes | L |
| 9 | auto-HR index | `0x2F` | 0 | L |
| 10 | live HR a | `0x81` | bpm / DND times | L |
| 11 | live HR b | `0x82` | bpm / quick-view times | L |
| 12 | time system | `0x27` | 24h | L |
| 13 | quiet hours | `0x2C` | 00:00–07:00 | L |
| 14 | watch-face layout | `0x39` | 240 × 284, ORIGINAL | L |
| 15 | goals | `0xB9{10}` | type/LE32 pairs | L |
| 16 | stress | `0xB9{11}` | state/timestamp/hourly | L |
| 17 | alarms | `0xB9{15}` | count + 8-byte records | L |
| 18 | message-app list | `0xB9{14,00}` | 19 enabled | L |
| 19 | move reminders | `0x83` | 50 steps / 60 min · 10:00–22:00 | L |
| 20 | step goal readback | `0x26` | LE32 10000 | L |

## Registry promotion evidence (L)

Captures from the 2026-11-22 sweep and 2026-11-23 endpoint probe that back `Live` entries in
`crates/core/src/protocol/registry.rs`. The harness reproduces and extends these on each live run.

| Opcode | Capability | Captured frame | Source |
|---|---|---|---|
| `0x26` | read step goal | `feea20092610270000` | 2026-11-23 endpoint probe |
| `0x83` | move reminders | `feea2009833c320a16` | 2026-11-23 endpoint probe |
| `0xB9` | message-app list (B9 14 00) | `feea201bb9140013000102030405060708090a0b0c0d0e0f111f80` | 2026-11-23 endpoint probe |
| `0x71` | set DND | `feea20097116000600` | 2026-11-22 sweep |
| `0x1F` | set auto-HR | `feea20061f01` | 2026-11-22 sweep |
| `0x16` | set step goal | `feea20091600002710` | 2026-11-22 sweep |
| `0x31` | time sync | `feea200a31f889026b08` | 2026-11-22 sweep |
| `0x41` | message push | `feea201641804f70656e576174636820766572696679` | 2026-11-22 sweep |
| `0x6D` | live HR trigger | `feea20066d00` | 2026-11-22 sweep |
| `0xB9` | set alarm (B9 05 00) | `feea200fb90500000101071eb10100` | 2026-11-22 sweep |
