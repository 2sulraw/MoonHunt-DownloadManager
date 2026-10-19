# Changelog

All notable changes to MoonHunt are documented here.
Older releases are described in the [GitHub Releases](https://github.com/2sulraw/MoonHunt-DownloadManager/releases).

## 0.0.6 — 2026-10-20

### Added

- **Addons system** — new Addon section in the sidebar:
  - Install community addons from a `.zip`/folder/URL; enable, disable, uninstall
  - Addons run in the app's webview and can extend MoonHunt using yt-dlp search
    and network fetch APIs provided by the host
  - URLs in the add-download dialog are matched against addon patterns — shows
    "Handled by *addon*" with an **Open in Addon** button
  - Export an addon template or package an addon folder as a distributable zip
- **Progress engine setting** — choose between Legacy and the new Experimental
  engine (fixed-denominator byte counting, higher-frequency progress ticks,
  merged phase updates) via a sliding segmented toggle in Settings
- **Download diagnostics** (opt-in checkbox in Settings) — engine name, phase,
  raw/proportional byte counters, last event, and a backend process-alive
  check, shown in the details Log tab
- **Connection-drop detection** — the log records a `timeout` event when the
  byte stream freezes for 20s and a `connection restored` event when it
  recovers; the packet-health ring now reflects timeout count
- **Aria2c settings category** — live status pill, binary location, engine
  start/restart/stop controls

### Fixed

- X/Twitter posts with multiple videos failed with *"Failed to parse yt-dlp
  JSON: trailing characters"* — now only the first JSON object is parsed
- yt-dlp downloads were mislabeled "file removed" when the file actually
  existed; filename matching now falls back through several recovery layers
  (fuzzy/sanitized matching, persisted real path)
- Pause/resume/cancel for yt-dlp downloads now work properly (pause no longer
  deletes the record, cancel kills the process, no duplicate cards on resume)
- Orphan sweep no longer flags live yt-dlp processes, so rows no longer flash
  stopped→error→running
- The log now reports the actually-selected quality/format instead of always
  claiming "best"
- Tab-switch lag: per-poll console spam removed, refreshes debounced,
  file-state checks batched, download grouping sped up, Media tab loads its
  IPC calls in parallel
- Schedule modal's date/time picker clipped behind the save button
- Search preview didn't reopen after clicking a suggestion (focus desync in
  the title bar)
- CSP updated to allow `blob:` scripts (required by the addon runtime)

### Changed

- **Light mode completely reworked** — neutral paper-white palette (white
  cards, cool-gray canvas, visible hairline borders, tuned slate text, softer
  shadows) and all light-mode accent variants re-tuned to keep WCAG AA contrast
- **Details modal reworked** — History tab folded into the Log tab with a
  merged timeline; compact diagnostics table; long filenames/paths/URLs
  truncated with tooltips; slimmer header/footer
- Proxy section in the details modal: **Apply** button lives inside the Enable
  Proxy card, toggling it off saves silently, auto-detect proxy button, equal
  2×2 input grid, new input colors (theme-aware in light mode)
- RPC configuration moved from Network to the new Aria2c tab
- Active Connections shown as a compact chip
