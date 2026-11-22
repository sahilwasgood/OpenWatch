# Endpoint probe — 2026-11-23

Live probe against Fire-Boltt 039 (`67:31:E4:DD:50:7A`). Raw capture: `endpoint-probe-2026-11-23.json`.
Each query-shaped frame was written to FEE2 with a 0.75 s reply window. "No reply" means the watch silently
ignored that frame — it does not prove the opcode is unsupported in every payload form.

## Newly recovered (now live-verified)

| Query | Reply | Interpretation |
|---|---|---|
| `0x83` | `feea2009833c320a16` | payload `3c 32 0a 16` = 60 min interval, 50 steps, 10:00→22:00 — move reminders, matches the ledger. |
| `0x85` | `feea201485000f1c05060e000000000000000001` | 15-byte women's-health record: `00 0f 1c 05 06 0e 00…01`. Layout not fully decoded; raw captured. |
| `B9{14,00}` | `feea201bb9140013000102030405060708090a0b0c0d0e0f111f80` | sub 0x14 = 20 (decimal). index 0, count `0x13` = 19, then 19 entries: `00 01 … 0f 11 1f 80`. This is the message-app list the design called `B9{20,0}` — now captured. |
| `B9{01,00}` | `feea2007530eff` | reply arrives on cmd `0x53` with payload `0e ff`; opcode family mismatch, treat as a generic status reply. |
| `B9{16,00}` | `feea2008b91100ff` | returns `B9{11,00,FF}` rather than echoing sub 0x16. |
| `0x2A` | `feea20062a00` | single-byte status `00`. |
| `0x2D` | `feea20062d00` | single-byte status `00`. |
| `0x26` | `feea20092610270000` | LE32 `10000` — confirms step-goal query endianness. |

## Silently ignored (no reply in this form)
`0x25`, `0x21`, `0x3A`, `0x34`, `0x62`, `0x66` (empty and `{01}`), `0xB6` (`{00,0000}`, `{01,0000}`, `{02,0000}`),
`0x64`, `0x74`, `0xB4`, `0x19`, `0x30`, `0x3B`, `0x3C`, and B9 subs `0x02`, `0x08`, `0x19`, `0x1B`, `0x1C`, `0x1D`.

These stay tagged `U` in the registry. Silence from a zero-payload probe is not proof of absence — a correctly
formed payload may still be required (as it was for `B9{14,00}`).

## Impact
- `0x83` and `B9{14,00}` can be promoted from `U`/`S` to `L` in the decoder registry with these captured fixtures.
- `0x85` has a captured frame but its field layout remains undecoded; keep `S` for field semantics.
- No payload guesses should be added for the silent opcodes without further source recovery.
