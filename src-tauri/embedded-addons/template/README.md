# MoonHunt Addon Template

Author addons as a folder with two files (no build step):

```
my-addon/
  manifest.json   copy this folder's manifest.json and edit it
  addon.js        copy addon.js and make it yours
```

## manifest.json

| field | rules |
|---|---|
| `id` | 2-64 chars, lowercase letters/digits/dashes, unique. `general` and `builtin` are reserved |
| `name` | 1-80 chars, shown as the sidebar tab |
| `version` | semver, e.g. `1.0.0` |
| `entry` | must be a `.js` file inside the package (usually `addon.js`) |
| `icon` | Material icon name for the tab (e.g. `library_music`) |
| `url_patterns` | hostname fragments, e.g. `["open.spotify.com"]`. Pasted download links matching these offer "Open in Addon" |
| `permissions` | shown at install for transparency (currently display-only) |

## addon.js

Single ESM file, default-export a Vue component (Options API + `render()`).
No SFC, no npm imports. Everything comes from the injected `host` prop:

- `host.h / ref / reactive / computed / watch / onMounted / onBeforeUnmount` (Vue primitives)
- `host.t(key)`, `host.notify(msg, 'success'|'error'|'info')`
- `host.invoke(cmd, args)` — any Tauri command, full privileges
- `host.fetchUrl(url)` — page text without CORS limits
- `host.ytSearch(query)` — first YouTube match `{ id, title, duration, uploader, webpage_url }`
- `host.createGroup(name)` / `host.addToGroup(groupId, gid)` — Downloads page groups
- `host.cancelDownload(gid)` — same as Cancel in Downloads; only for still-active downloads

Optional: `methods.loadUrl(url)` receives links sent from the
download manager; prop `initialUrl` carries a link on first open.

Use the global `nd-*` classes (`nd-card`, `nd-btn`, `nd-input`, …)
so your addon looks native.

## Test loop

Addon page → Add → **Install from folder** → pick your folder.
Iterate without zipping. Share by zipping the folder
(`manifest.json` at the zip root); others use **Install from file/URL**.

Only install addons you trust: addon code runs inside MoonHunt
with full app privileges.
