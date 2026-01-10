# Time Ledger TUI

Track time spent on tasks in a weekly view using a plain-text ledger file.

## Run

```bash
cargo run
```

The app loads the current week from `data/YYYY-Www.ledger` (ISO week, Monday start).

## Controls

- `q` / `Esc`: quit
- `←` / `→`: move day selection
- `↑` / `↓`: move task selection
- `s`: save (normalizes time, fills missing parent times from sub-items, regenerates totals)

## Time format

- Allowed: `1d`, `1h`, `30m`, `1h 30m`, `1d 2h 15m`, `90m`
- Not allowed: `1:30`
- Normalization rules on save:
  - `90m` -> `1h 30m`
  - `8h` -> `1d`
  - `1d 8h` -> `2d`
  - Units are always written as `d h m` with spaces.

## Ledger file format

```text
# Week 2025-12-29

## 2025-12-29 Mon
- Client A @2h [x]
  - kickoff @30m
  - notes @1h 30m
- Build pipeline @45m
- Admin @1h 15m
;; item-total Client A @2h
;; item-total Build pipeline @45m
;; item-total Admin @1h 15m
;; day-total @4h

;; week-total @4h
;; week-item-total Admin @1h 15m
;; week-item-total Build pipeline @45m
;; week-item-total Client A @2h
```

### Notes

- `[x]` is a manual “counted” marker.
- If a parent has no time but all sub-items do, the parent time is computed and written on save.
- Totals are generated lines starting with `;;` and are safe to overwrite.
