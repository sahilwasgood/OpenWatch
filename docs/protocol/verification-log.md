# OpenWatch — live verification log

Device: Fire-Boltt 039 (`67:31:E4:DD:50:7A`), firmware `1.0.40`.
Session: 2026-11-22, battery `168` = 68% + charging flag.
Raw capture: `live-verify-2026-11-22.json`. Read sweep: `fireboltt_sweep_1795337286.json`.

All writes below were sent on the real watch. State-changing commands were read back and
restored to their original values where practical.

| Capability | Opcode | TX frame | Readback / result | Confidence |
|---|---|---|---|---|
| DND set | `0x71` | `feea20097116000600` (22:00→06:00) | `0x81` → `28056801` = 22:00/06:00 | **L** |
| DND restore | `0x71` | `feea20097100000000` | `0x81` → `00000000` (off, original) | **L** |
| Auto-HR set | `0x1F` | `feea20061f01` | `0x2F` → `01` | **L** |
| Auto-HR restore | `0x1F` | `feea20061f00` | back to `00` | **L** |
| Alarm set | `0xB9{05,00,…}` | `feea200fb90500000101071eb10100` | dump `feea2010b9150401000101071eb10100` = count 1 + record | **L** |
| Alarm delete | `0xB9{05,02,00}` | `feea2008b9050200` | dump `feea2008b9150400` = count 0 | **L** |
| Weather today | `0x43` | `feea20184300051c002000200020002000440045004c0048` | accepted, no NACK | **S** (on-glass unverified) |
| Weather forecast | `0x42` | `feea201d42052018031f17079c9c079c9c079c9c079c9c079c9c079c9c` | accepted, no NACK | **S** |
| Weather sun | `0xB5` | `feea2013b500051c0000060c122f44454c4849` | accepted, no NACK | **S** |
| Time sync | `0x31` | `feea200a31f889026b08` | fire-and-forget, no ACK | **L** |
| Message push | `0x41` | `feea201641804f70656e576174636820766572696679` | transient, no ACK | **L** |
| Live HR trigger | `0x6D` | `feea20066d00` | `feea20066dff` = busy (unworn) | **L** |
| Step goal | `0x16` | `feea20091600002710` | `0x26` → `10270000` LE32 = 10000; watch recomputes derived goal type 3 | **L** |

## Notes

- `0x81` replies carry **two** LE16 times (DND start/end); `0x82` carries quick-view start/end.
  The Python sweep decoder only reads the first, so raw frames must be consulted for the pair.
- Goal type 3 moved from 7500 to 8300 when step goal was written; treat it as device-derived.
- `0x64` weather requests were not observed in 75 s of idle listening on a bare connection.
- Legacy opcodes (`0x66` camera, `0x83` move reminders, `0x85` women's health, `0xB6` day-pack)
  have no recoverable byte layouts in the workspace and remain **U** — do not guess payloads.
