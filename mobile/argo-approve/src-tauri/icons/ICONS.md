# App icons

This shell does not ship platform-specific icons.

Run `tauri icon path/to/source.png` to generate the full
icon set (32x32, 128x128, 128x128@2x, icon.png, plus
iOS / Android variants under `gen/`).

The bundle entry in `tauri.conf.json` references:
- `icons/32x32.png`
- `icons/128x128.png`
- `icons/icon.png`

These must exist before `tauri build` succeeds.
