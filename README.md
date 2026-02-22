# Time Ledger TUI

Track time spent on tasks in a weekly view using a plain-text ledger file.

## Run

```bash
cargo run
```

The app loads the current week from `<ledger-dir>/YYYY-Www.ledger` (ISO week, Monday start).

You can run the app immediately without pre-creating a ledger file. If the current week's file does not exist yet, edit any day and save (`Ctrl+s` in day edit mode), and the app will create the ledger automatically using the correct ISO year/week filename.

To load ledgers from another directory for a single run:

```bash
cargo run -- --ledger-dir /path/to/ledger-dir
```

To configure a default ledger directory, create:

`~/.config/time-ledger/config.toml`

```toml
ledger_dir = "/absolute/path/to/ledger-dir"
```

Resolution order:

1. `--ledger-dir DIR`
1. `$XDG_CONFIG_HOME/time-ledger/config.toml` (if set)
1. `~/.config/time-ledger/config.toml`

If no `ledger_dir` is provided through CLI or config, the app exits with an explicit error instead of creating a default `data/` directory.

To print the ISO week number (`Wxx`) for today:

```bash
cargo run -- --week-number
```

To print the ISO week number for a specific date:

```bash
cargo run -- --week-number 2026-02-08
```

## Controls

### Main screen

- `q` / `Esc`: quit
- `←` / `→`: move day selection
- `↑` / `↓`: move task selection
- `s`: save (normalizes time, fills missing parent times from sub-items, regenerates totals)
- `e`: open in-place editor for the selected day
- `w`: open warnings overlay

### Day edit mode

- Type directly in ledger line format for the selected day
- `Esc`: leave edit mode
- `Ctrl+s`: validate and save current day edit
- `←` / `→` / `↑` / `↓`: move cursor
- Word jump:
  - macOS: `Option+←` / `Option+→`
  - non-macOS: `Ctrl+←` / `Ctrl+→`
- Diagnostics panel navigation:
  - `PgUp` / `PgDn`: scroll diagnostics
  - `Home` / `End`: jump to start/end of diagnostics

### Warnings overlay

- `w`, `q`, or `Esc`: close overlay
- `↑` / `↓`: scroll
- `PgUp` / `PgDn`: page scroll
- `Home` / `End`: jump to start/end

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
- Day details in the right pane are shown in the same parent/sub-item structure as the ledger text.
- In edit mode, diagnostics include parse errors and parent/sub-item time mismatches; saving is blocked until issues are fixed.
