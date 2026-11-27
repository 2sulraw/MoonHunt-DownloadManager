// MoonHunt addon template (community format).
//
// Rules:
// - Single ESM file. Default-export a Vue component (Options API recommended).
// - No .vue files, no build step, no bare imports ('vue', npm packages).
//   Use the injected `host` prop instead:
//     host.h, host.ref, host.reactive, host.computed, host.watch,
//     host.onMounted, host.onBeforeUnmount,
//     host.t(key)            -> translated string (falls back to key)
//     host.notify(msg, type) -> 'success' | 'error' | 'info' toast
//     host.invoke(cmd, args) -> any Tauri command (full app privileges!)
//     host.fetchUrl(url)     -> page text, CORS-free (addon_fetch_url)
//     host.ytSearch(query)   -> first YouTube match (addon_yt_search)
//     host.createGroup(name) -> { id, name } download group (Download page)
//     host.addToGroup(groupId, gid) -> add a queued download to a group
//     host.cancelDownload(gid) -> same as Cancel in Downloads (active only!)
// - Styling: the app's shared `nd-*` classes are available globally
//   (nd-card, nd-btn, nd-input, ...). Prefer them over custom CSS.
// - Optional: `methods.loadUrl(url)` is called when the user sends a link
//   to your addon (e.g. from the download manager). Optional prop
//   `initialUrl` carries a link on first open.
// - manifest.json `url_patterns` decides which pasted links offer
//   "Open in Addon" (match against the link hostname).
//
// Test loop: Addon page -> Add -> "Install from folder" -> pick this folder.
// Share: zip manifest.json + addon.js (+ assets) -> others install the zip.

export default {
  name: 'MyAddon',
  props: {
    host: { type: Object, required: true },
    initialUrl: { type: String, default: '' },
  },
  data() {
    return {
      count: 0,
      lastSearch: '',
      searching: false,
    }
  },
  methods: {
    loadUrl(url) {
      this.lastSearch = url
      this.hello()
    },
    hello() {
      this.count += 1
      this.host.notify(`Hello from My Addon (#${this.count})`, 'success')
    },
    async search() {
      if (!this.lastSearch.trim() || this.searching) return
      this.searching = true
      try {
        const res = await this.host.ytSearch(this.lastSearch.trim())
        this.host.notify(`Top match: ${res.title}`, 'info')
      } catch (e) {
        this.host.notify(`Search failed: ${String(e)}`, 'error')
      } finally {
        this.searching = false
      }
    },
  },
  render() {
    const { h } = this.host
    return h('section', { class: 'nd-card' }, [
      h('header', { class: 'nd-card-header' }, [
        h('div', { class: 'nd-card-head-left' }, [
          h('div', { class: 'nd-avatar' }, [h('i', { class: 'material-icons' }, 'extension')]),
          h('div', { class: 'nd-titles' }, [
            h('h2', { class: 'nd-title' }, 'My Addon'),
            h('span', { class: 'nd-subtitle' }, 'Counter + YouTube search demo. Make it yours.'),
          ]),
        ]),
      ]),
      h('div', { class: 'nd-divider' }),
      h('div', { class: 'nd-card-body' }, [
        h('div', { class: 'nd-field' }, [
          h('label', { class: 'nd-label' }, 'Query'),
          h('div', { class: 'nd-input-wrap' }, [
            h('input', {
              class: 'nd-input',
              placeholder: 'lofi hip hop mix',
              value: this.lastSearch,
              onInput: (e) => { this.lastSearch = e.target.value },
              onKeyup: (e) => { if (e.key === 'Enter') this.search() },
            }),
          ]),
        ]),
        h('div', { style: 'display:flex;gap:8px;flex-wrap:wrap' }, [
          h('button', { class: 'nd-btn nd-btn-primary', style: 'flex:0 1 auto', onClick: () => this.hello() }, 'Say hello'),
          h('button', {
            class: 'nd-btn nd-btn-secondary',
            style: 'flex:0 1 auto',
            disabled: this.searching || !this.lastSearch.trim(),
            onClick: () => this.search(),
          }, this.searching ? 'Searching...' : 'Search YouTube'),
        ]),
      ]),
    ])
  },
}
