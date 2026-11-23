# OpenWatch — generated live verification log

Generated: 2026-11-23 13:13:01 UTC (1795439581)

Device: Fire-Boltt 039 (`67:31:E4:DD:50:7A`)

Battery read: `95` = 95%

Produced by the ignored live harness (`crates/core/tests/live/verify.rs`).
Generated only with `OPENWATCH_LIVE_VERIFY=1 ... --ignored`, never by `cargo test`.
State-changing writes record before/after reads and restore originals where practical.
No face upload / DFU / AGPS / OTA commands are sent.

## Captured frames

| UTC | Dir | Capability | Opcode | Frame | Decoded / readback | Conf |
|---|---|---|---|---|---|---|
| 2026-11-23 13:12:33 UTC | RX | battery | `2A19` | `5f` | 95% | L |
| 2026-11-23 13:12:33 UTC | TX | steps · query | `0x2B` | `feea20052b` |  | L |
| 2026-11-23 13:12:33 UTC | RX | steps | `0x2B` | `feea200e2b0000001df300000000` | Number(7667) | L |
| 2026-11-23 13:12:33 UTC | TX | language · query | `0x2E` | `feea20052e` |  | L |
| 2026-11-23 13:12:34 UTC | RX | language | `0x2E` | `feea20082e010000` | Text("international") | L |
| 2026-11-23 13:12:34 UTC | TX | firmware · query | `0xBA` | `feea2005ba` |  | L |
| 2026-11-23 13:12:35 UTC | RX | firmware | `0xBA` | `feea2008ba010028` | Text("1.0.40") | L |
| 2026-11-23 13:12:35 UTC | TX | edition · query | `0x29` | `feea200529` |  | L |
| 2026-11-23 13:12:36 UTC | RX | edition | `0x29` | `feea20062906` | Bytes([6]) | L |
| 2026-11-23 13:12:36 UTC | TX | message limit · query | `0xBB` | `feea2005bb` |  | L |
| 2026-11-23 13:12:36 UTC | RX | message limit | `0xBB` | `` | no reply (no matching reply received) | L |
| 2026-11-23 13:12:36 UTC | TX | device state · query | `0x84` | `feea200584` |  | L |
| 2026-11-23 13:12:37 UTC | RX | device state | `0x84` | `feea200584` | Bytes([]) | L |
| 2026-11-23 13:12:37 UTC | TX | HR history · query | `0x35` | `feea200535` |  | L |
| 2026-11-23 13:12:38 UTC | RX | HR history | `0x35` | `feea204e3500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000` | Bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) | L |
| 2026-11-23 13:12:38 UTC | TX | step categories · query | `0x59` | `feea200559` |  | L |
| 2026-11-23 13:12:39 UTC | RX | step categories | `0x59` | `feea20365900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000` | Bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) | L |
| 2026-11-23 13:12:39 UTC | TX | auto-HR index · query | `0x2F` | `feea20052f` |  | L |
| 2026-11-23 13:12:39 UTC | RX | auto-HR index | `0x2F` | `feea20062f00` | Bytes([0]) | L |
| 2026-11-23 13:12:39 UTC | TX | live HR a · query | `0x81` | `feea200581` |  | L |
| 2026-11-23 13:12:40 UTC | RX | live HR a | `0x81` | `feea20098100000000` | Bytes([0, 0, 0, 0]) | L |
| 2026-11-23 13:12:40 UTC | TX | live HR b · query | `0x82` | `feea200582` |  | L |
| 2026-11-23 13:12:41 UTC | RX | live HR b | `0x82` | `feea200982a4012805` | Bytes([164, 1, 40, 5]) | L |
| 2026-11-23 13:12:41 UTC | TX | time system · query | `0x27` | `feea200527` |  | L |
| 2026-11-23 13:12:42 UTC | RX | time system | `0x27` | `feea20062701` | Text("24h") | L |
| 2026-11-23 13:12:42 UTC | TX | quiet hours · query | `0x2C` | `feea20052c` |  | L |
| 2026-11-23 13:12:42 UTC | RX | quiet hours | `0x2C` | `feea20062c00` | Bytes([0]) | L |
| 2026-11-23 13:12:42 UTC | TX | watch-face layout · query | `0x39` | `feea200539` |  | L |
| 2026-11-23 13:12:43 UTC | RX | watch-face layout | `0x39` | `feea203339000104ffff000000000000000000000000000000000000000000000000000000000000000000f0008c011c00a3ff` | Bytes([0, 1, 4, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 0, 140, 1, 28, 0, 163, 255]) | L |
| 2026-11-23 13:12:43 UTC | TX | goals · query | `0xB9{10}` | `feea2006b910` |  | L |
| 2026-11-23 13:12:44 UTC | RX | goals | `0xB9{10}` | `feea201bb910000110270000022c010000036c200000041e000000` | Bytes([16, 0, 1, 16, 39, 0, 0, 2, 44, 1, 0, 0, 3, 108, 32, 0, 0, 4, 30, 0, 0, 0]) | L |
| 2026-11-23 13:12:44 UTC | TX | stress · query | `0xB9{11}` | `feea2006b911` |  | L |
| 2026-11-23 13:12:45 UTC | RX | stress | `0xB9{11}` | `` | no reply (no matching reply received) | L |
| 2026-11-23 13:12:45 UTC | TX | alarms · query | `0xB9{15}` | `feea2006b915` |  | L |
| 2026-11-23 13:12:45 UTC | RX | alarms | `0xB9{15}` | `` | no reply (no matching reply received) | L |
| 2026-11-23 13:12:45 UTC | TX | message-app list · query | `0xB9{1400}` | `feea2007b91400` |  | L |
| 2026-11-23 13:12:46 UTC | RX | message-app list | `0xB9{1400}` | `feea201bb9140013000102030405060708090a0b0c0d0e0f111f80` | Bytes([20, 0, 19, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 31, 128]) | L |
| 2026-11-23 13:12:46 UTC | TX | move reminders · query | `0x83` | `feea200583` |  | L |
| 2026-11-23 13:12:47 UTC | RX | move reminders | `0x83` | `feea2009833c320a16` | Text("50 steps every 60 min · 10:00–22:00") | L |
| 2026-11-23 13:12:47 UTC | TX | step goal readback · query | `0x26` | `feea200526` |  | L |
| 2026-11-23 13:12:48 UTC | RX | step goal readback | `0x26` | `feea20092610270000` | raw 10270000 | L |
| 2026-11-23 13:12:48 UTC | TX | step goal · before · query | `0x26` | `feea200526` |  | L |
| 2026-11-23 13:12:48 UTC | RX | step goal · before | `0x26` | `feea20092610270000` | raw 10270000 | L |
| 2026-11-23 13:12:48 UTC | TX | set step goal | `0x16` | `feea20091600002710` | write | L |
| 2026-11-23 13:12:49 UTC | TX | step goal · after · query | `0x26` | `feea200526` |  | L |
| 2026-11-23 13:12:50 UTC | RX | step goal · after | `0x26` | `feea20092610270000` | raw 10270000 | L |
| 2026-11-23 13:12:50 UTC | TX | step goal · restore | `0x16` | `feea20091600002710` | write | L |
| 2026-11-23 13:12:50 UTC | TX | time system · before · query | `0x27` | `feea200527` |  | L |
| 2026-11-23 13:12:51 UTC | RX | time system · before | `0x27` | `feea20062701` | Text("24h") | L |
| 2026-11-23 13:12:51 UTC | TX | set time system | `0x17` | `feea20061700` | write | L |
| 2026-11-23 13:12:51 UTC | TX | time system · after · query | `0x27` | `feea200527` |  | L |
| 2026-11-23 13:12:52 UTC | RX | time system · after | `0x27` | `feea20062700` | Text("12h") | L |
| 2026-11-23 13:12:52 UTC | TX | time system · restore | `0x17` | `feea20061701` | write | L |
| 2026-11-23 13:12:52 UTC | TX | DND · before · query | `0x81` | `feea200581` |  | L |
| 2026-11-23 13:12:53 UTC | RX | DND · before | `0x81` | `feea20098100000000` | Bytes([0, 0, 0, 0]) | L |
| 2026-11-23 13:12:53 UTC | TX | set DND 22:00-06:00 | `0x71` | `feea20097116000600` | write | L |
| 2026-11-23 13:12:53 UTC | TX | DND · after · query | `0x81` | `feea200581` |  | L |
| 2026-11-23 13:12:54 UTC | RX | DND · after | `0x81` | `feea20098128056801` | Bytes([40, 5, 104, 1]) | L |
| 2026-11-23 13:12:54 UTC | TX | DND · restore | `0x71` | `feea20097100000000` | write | L |
| 2026-11-23 13:12:54 UTC | TX | alarms · before · query | `0xB9{15}` | `feea2006b915` |  | L |
| 2026-11-23 13:12:55 UTC | RX | alarms · before | `0xB9{15}` | `` | no reply (no matching reply received) | L |
| 2026-11-23 13:12:55 UTC | NOTE | alarm set/delete | `0xB9{05}` | `` | skipped: 255 alarm(s) already configured; do not overwrite | L |
| 2026-11-23 13:12:55 UTC | TX | weather today | `0x43` | `feea20184300051c002000200020002000440045004c0048` | write | S |
| 2026-11-23 13:12:55 UTC | TX | weather forecast | `0x42` | `feea201d42052018031f17079c9c079c9c079c9c079c9c079c9c079c9c` | write | S |
| 2026-11-23 13:12:56 UTC | TX | weather sun | `0xB5` | `feea2013b500051c0000060c122f44454c4849` | write | S |
| 2026-11-23 13:12:56 UTC | TX | message push | `0x41` | `feea201641804f70656e576174636820766572696679` | write | L |
| 2026-11-23 13:12:57 UTC | TX | time sync | `0x31` | `feea200a31d93b046b08` | write | L |
| 2026-11-23 13:12:57 UTC | TX | auto-HR · before · query | `0x2F` | `feea20052f` |  | L |
| 2026-11-23 13:12:58 UTC | RX | auto-HR · before | `0x2F` | `feea20062f00` | Bytes([0]) | L |
| 2026-11-23 13:12:58 UTC | TX | set auto-HR on | `0x1F` | `feea20061f01` | write | L |
| 2026-11-23 13:12:58 UTC | TX | auto-HR · after · query | `0x2F` | `feea20052f` |  | L |
| 2026-11-23 13:12:59 UTC | RX | auto-HR · after | `0x2F` | `feea20062f01` | Bytes([1]) | L |
| 2026-11-23 13:12:59 UTC | TX | auto-HR · restore | `0x1F` | `feea20061f00` | write | L |
| 2026-11-23 13:12:59 UTC | TX | live HR · query | `0x6D` | `feea20066d00` |  | L |
| 2026-11-23 13:13:00 UTC | RX | live HR | `0x6D` | `` | no reply (no matching reply received) | L |
| 2026-11-23 13:13:00 UTC | TX | camera (legacy probe) · query | `0x66{01}` | `feea20066601` |  | U |
| 2026-11-23 13:13:01 UTC | RX | camera (legacy probe) | `0x66{01}` | `` | no reply (no matching reply received) | U |
| 2026-11-23 13:13:01 UTC | TX | ring (legacy probe) · query | `0x62{01}` | `feea20066201` |  | U |
| 2026-11-23 13:13:01 UTC | RX | ring (legacy probe) | `0x62{01}` | `` | no reply (no matching reply received) | U |

## Registry promotion evidence (L)

Baseline captures from the 2026-11-22 sweep and 2026-11-23 endpoint probe. The harness reproduces and extends these on each live run.

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
