// moonhunt-Spotify-playlist-bridge v0.0.3 — reference addon (community format).
// Paste a Spotify track/playlist/album link, match tracks on YouTube
// via yt-dlp, download each song or the whole list.
//
// No Spotify API is used: single tracks resolve via the public oEmbed
// endpoint, playlists via the embedded page data (fragile — Spotify may
// change markup). A manual "Artist - Title" paste fallback is included.

function trackArtist(o) {
  if (Array.isArray(o.artists)) {
    return o.artists.map((x) => (typeof x === 'string' ? x : x && x.name)).filter(Boolean).join(', ')
  }
  return typeof o.subtitle === 'string' ? o.subtitle : ''
}

function trackDurationMs(o) {
  // Embed pages use milliseconds in `duration`; other shapes use `duration_ms`.
  if (typeof o.duration === 'number') return o.duration > 10000 ? o.duration : Math.round(o.duration * 1000)
  if (typeof o.duration_ms === 'number') return o.duration_ms
  return undefined
}

function isTrackLike(o) {
  return o && typeof o === 'object' &&
    typeof o.title === 'string' && o.title.trim() &&
    trackArtist(o).trim() !== ''
}

// Best-effort track extraction. NOTE: plain open.spotify.com pages serve a
// bot shell with no data, so resolve() fetches the /embed/ page, whose
// __NEXT_DATA__ carries props.pageProps.state.data.entity.trackList
// ([{title, subtitle, duration(ms), uri}]) for playlists/albums, or a lone
// entity with title/subtitle for single tracks.
function extractSpotifyTracks(html) {
  const out = []
  const push = (o) => {
    out.push({
      title: o.title.trim(),
      artist: trackArtist(o).trim() || 'Unknown artist',
      durationMs: trackDurationMs(o),
    })
  }
  const m = html.match(/<script id="__NEXT_DATA__" type="application\/json">([\s\S]*?)<\/script>/)
  if (m) {
    try {
      const data = JSON.parse(m[1])
      const seen = new Set()
      const LIST_KEYS = ['tracklist', 'tracks', 'items', 'track_list']
      const walk = (node, key) => {
        if (!node || typeof node !== 'object' || seen.has(node)) return
        seen.add(node)
        if (Array.isArray(node)) {
          const allTracks = node.length && node.every(isTrackLike)
          if (allTracks) {
            const named = LIST_KEYS.includes(String(key).toLowerCase())
            const evidenced = node.every((it) =>
              typeof it.duration === 'number' ||
              typeof it.duration_ms === 'number' ||
              (typeof it.uri === 'string' && it.uri.startsWith('spotify:track')))
            if (named || evidenced) {
              for (const it of node) push(it)
              return // don't recurse into a consumed list
            }
          }
          for (let i = 0; i < node.length; i++) walk(node[i], i)
        } else {
          // Lone playable entity (single-track embed pages).
          if (typeof node.uri === 'string' && node.uri.startsWith('spotify:track') && isTrackLike(node)) {
            push(node)
          }
          for (const k of Object.keys(node)) walk(node[k], k)
        }
      }
      walk(data, 'root')
    } catch {
      /* fall through to meta fallback */
    }
  }
  if (!out.length) {
    const ogTitle = (html.match(/<meta property="og:title" content="([^"]+)"/) || [])[1]
    const ogDesc = (html.match(/<meta property="og:description" content="([^"]+)"/) || [])[1]
    if (ogTitle) {
      const artist = (ogDesc || '').split('·')[0].trim()
      if (ogTitle.replace(/&amp;/g, '&').trim()) {
        out.push({ title: ogTitle.replace(/&amp;/g, '&').trim(), artist: artist || 'Unknown artist' })
      }
    }
  }
  const seen = new Set()
  return out.filter((x) => {
    const k = `${x.artist} - ${x.title}`.toLowerCase()
    if (seen.has(k)) return false
    seen.add(k)
    return true
  })
}

function fmtDur(totalSeconds) {
  const m = Math.floor(totalSeconds / 60)
  const s = Math.round(totalSeconds % 60)
  return `${m}:${String(s).padStart(2, '0')}`
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

const SEARCH_MODES = [
  { key: 'legacy', label: 'Legacy', hint: 'Sequential, one at a time. Recommended for restricted networks.' },
  { key: 'p2', label: '2x', hint: '2 parallel searches.' },
  { key: 'p3', label: '3x', hint: '3 parallel searches.' },
  { key: 'all', label: 'All', hint: 'Everything at once. Experimental — may hit YouTube rate limits.' },
]

function loadSearchMode() {
  try {
    const v = localStorage.getItem('spm-search-mode')
    if (SEARCH_MODES.some((m) => m.key === v)) return v
  } catch {
    /* storage unavailable */
  }
  return 'legacy'
}

// Playlist/album title for group naming: embed __NEXT_DATA__ entity name.
function extractPlaylistName(html) {
  try {
    const m = html.match(/<script id="__NEXT_DATA__" type="application\/json">([\s\S]*?)<\/script>/)
    if (!m) return ''
    const data = JSON.parse(m[1])
    const entity = data && data.props && data.props.pageProps &&
      data.props.pageProps.state && data.props.pageProps.state.data &&
      data.props.pageProps.state.data.entity
    if (entity && typeof entity.name === 'string' && entity.name.trim()) return entity.name.trim()
  } catch {
    /* unknown layout */
  }
  return ''
}

// /playlist/<id> etc. -> https://open.spotify.com/embed/<type>/<id>.
// The embed page server-renders track data; the normal page is a bot shell.
function embedUrl(u) {
  const m = String(u).match(/open\.spotify\.com\/(playlist|track|album)\/([A-Za-z0-9]+)/i)
  if (!m) return ''
  return `https://open.spotify.com/embed/${m[1].toLowerCase()}/${m[2]}`
}

export default {
  name: 'MoonhuntSpotifyPlaylistBridge',
  props: {
    host: { type: Object, required: true },
    initialUrl: { type: String, default: '' },
  },
  data() {
    return {
      url: this.initialUrl || '',
      resolving: false,
      resolveError: '',
      resolveNote: '',
      playlistName: '',
      tracks: [],
      matching: false,
      matchCancel: false,
      downloadingAll: false,
      cancelRequested: false,
      audioFormat: 'mp3',
      searchMode: 'legacy',
      reqChecking: false,
      reqStatus: null,
      showManual: false,
      manualText: '',
    }
  },
  created() {
    this.searchMode = loadSearchMode()
    this.checkRequirements()
  },
  computed: {
    matchedCount() {
      return this.tracks.filter((x) => x.match).length
    },
    unmatchedCount() {
      return this.tracks.filter((x) => !x.match && x.matchState !== 'searching').length
    },
    readyCount() {
      return this.tracks.filter((x) => x.match && x.dlState !== 'done').length
    },
    queueTotal() {
      return this.tracks.filter((x) => x.match).length
    },
    queuedCount() {
      return this.tracks.filter((x) => x.match && (x.dlState === 'done' || x.dlState === 'starting')).length
    },
    queuePct() {
      if (!this.queueTotal) return 0
      return Math.round((this.tracks.filter((x) => x.match && x.dlState === 'done').length / this.queueTotal) * 100)
    },
  },
  watch: {
    initialUrl(v) {
      if (v) {
        this.url = v
        this.resolve()
      }
    },
  },
  methods: {
    loadUrl(u) {
      this.url = u
      this.resolve()
    },
    async resolve() {
      const u = (this.url || '').trim()
      if (!u) {
        this.host.notify('Please enter a URL', 'error')
        return
      }
      await this.checkRequirements()
      if (this.reqStatus && !this.reqStatus.ytdlp) {
        this.resolveError = 'yt-dlp is not installed — set it up on the Media page first.'
        return
      }
      this.resolving = true
      this.resolveError = ''
      this.resolveNote = ''
      try {
        if (!/spotify\.com/i.test(u)) {
          this.resolveError = 'Only Spotify links are supported here.'
          return
        }
        // Single-track fast path: public oEmbed. NOTE: oEmbed also answers
        // for playlists/albums (with the *list* title and no author), so
        // only trust it for track/episode URLs.
        if (/\/(track|episode)\//i.test(u)) {
          try {
            const oembed = await this.host.fetchUrl(`https://open.spotify.com/oembed?url=${encodeURIComponent(u)}`)
            const meta = JSON.parse(oembed)
            if (meta && meta.title) {
              this.tracks = [{
                title: meta.title,
                artist: meta.author_name || 'Unknown artist',
                matchState: 'pending',
                dlState: 'idle',
              }]
              this.resolveNote = 'Resolved 1 track via Spotify oEmbed.'
              this.matchAll()
              return
            }
          } catch {
            /* fall through to page scrape */
          }
        }
        const html = await this.host.fetchUrl(embedUrl(u) || u)
        const found = extractSpotifyTracks(html)
        if (!found.length) {
          this.resolveError = 'Could not read tracks from this Spotify page. Spotify may have changed its layout — use manual paste below.'
          this.showManual = true
          return
        }
        this.playlistName = extractPlaylistName(html)
        this.tracks = found.map((x) => ({ ...x, matchState: 'pending', dlState: 'idle' }))
        this.resolveNote = `Resolved ${found.length} tracks${this.playlistName ? ` from "${this.playlistName}"` : ''} (scraped embed data).`
        this.matchAll()
      } catch (e) {
        this.resolveError = String(e)
      } finally {
        this.resolving = false
      }
    },
    useManualList() {
      const lines = this.manualText.split('\n').map((l) => l.trim()).filter(Boolean)
      this.playlistName = 'Spotify tracks'
      this.tracks = lines.map((line) => {
        const parts = line.includes(' - ') ? line.split(/ - (.+)/) : ['', line]
        return {
          artist: (parts[0] || 'Unknown artist').trim(),
          title: (parts[1] || line).trim(),
          matchState: 'pending',
          dlState: 'idle',
        }
      })
      this.resolveError = ''
      this.showManual = false
      this.matchAll()
    },
    async matchOne(i) {
      const tr = this.tracks[i]
      if (!tr || tr.matchState === 'searching') return
      tr.matchState = 'searching'
      tr.matchError = ''
      try {
        tr.match = await this.host.ytSearch(`${tr.title} ${tr.artist}`)
        tr.matchState = 'ok'
      } catch (e) {
        tr.matchState = 'fail'
        tr.matchError = String(e).slice(0, 140)
      }
    },
    async matchAll() {
      if (this.matching) return
      this.matching = true
      this.matchCancel = false
      try {
        const queue = []
        for (let i = 0; i < this.tracks.length; i++) {
          const tr = this.tracks[i]
          if (!tr.match && tr.matchState !== 'searching') queue.push(i)
        }
        if (this.searchMode === 'legacy') {
          for (const i of queue) {
            if (this.matchCancel) break
            await this.matchOne(i)
            await sleep(400)
          }
        } else {
          const limit = this.searchMode === 'all'
            ? queue.length
            : this.searchMode === 'p3' ? 3 : 2
          let next = 0
          const worker = async (stagger) => {
            await sleep(stagger * 150)
            while (next < queue.length) {
              if (this.matchCancel) return
              const i = queue[next++]
              await this.matchOne(i)
            }
          }
          const workers = []
          for (let k = 0; k < Math.min(limit, queue.length); k++) workers.push(worker(k))
          await Promise.all(workers)
        }
      } finally {
        this.matching = false
        this.matchCancel = false
      }
    },
    cancelSearch() {
      this.matchCancel = true
    },
    // Media-page prerequisites, shown as a status strip. MP3 downloads need
    // a merger (FFmpeg); without one, matching works but extraction fails.
    // Pills name the effective runtime/merger: explicit selection wins,
    // otherwise the backend's automatic fallback (if usable).
    async checkRequirements() {
      if (this.reqChecking) return
      this.reqChecking = true
      try {
        const [ytdlp, runtimes, mergers] = await Promise.all([
          this.host.invoke('check_yt_dlp_exists').catch(() => false),
          this.host.invoke('get_js_runtimes').catch(() => null),
          this.host.invoke('get_mergers').catch(() => null),
        ])
        let runtimeName = ''
        let runtimeOk = false
        if (runtimes) {
          const list = runtimes.runtimes || []
          const sys = runtimes.systemNode || null
          const chosen = list.find((r) => r.selected && r.installed)
          if (sys && sys.selected) {
            runtimeName = 'System Node.js'
            runtimeOk = true
          } else if (chosen) {
            runtimeName = chosen.name || chosen.id
            runtimeOk = true
          } else if (sys) {
            runtimeName = 'System Node.js (auto)'
            runtimeOk = true
          } else if (list.some((r) => r.installed)) {
            runtimeName = 'Node.js (auto, PATH)'
            runtimeOk = true
          }
        }
        let mergerName = ''
        let mergerOk = false
        if (mergers) {
          const list = mergers.mergers || []
          const sys = mergers.systemFfmpeg || null
          const chosen = list.find((m) => m.selected && m.installed)
          const anyInstalled = list.find((m) => m.installed)
          if (sys && sys.selected) {
            mergerName = 'System FFmpeg'
            mergerOk = true
          } else if (chosen) {
            mergerName = chosen.name || chosen.id
            mergerOk = true
          } else if (anyInstalled) {
            mergerName = `${anyInstalled.name || anyInstalled.id} (auto)`
            mergerOk = true
          }
        }
        this.reqStatus = {
          ytdlp: ytdlp === true,
          runtime: runtimeOk,
          runtimeName,
          merger: mergerOk,
          mergerName,
        }
      } finally {
        this.reqChecking = false
      }
    },
    setSearchMode(mode) {
      if (!SEARCH_MODES.some((m) => m.key === mode)) return
      this.searchMode = mode
      try {
        localStorage.setItem('spm-search-mode', mode)
      } catch {
        /* storage unavailable */
      }
    },
    async downloadTrack(i) {
      const tr = this.tracks[i]
      if (!tr || !tr.match || tr.dlState === 'starting' || tr.dlState === 'done') return null
      tr.dlState = 'starting'
      try {
        const gid = await this.host.invoke('add_yt_download', {
          url: tr.match.webpage_url,
          format: this.audioFormat,
          quality: undefined,
          path: undefined,
          browser: undefined,
          format_id: undefined,
          has_video: undefined,
          has_audio: undefined,
        })
        tr.dlState = 'done'
        if (typeof gid === 'string') tr.gid = gid
        window.dispatchEvent(new CustomEvent('yt-dlp-start', { detail: { url: tr.match.webpage_url } }))
        this.host.notify(`Download started: ${tr.title}`, 'success')
        return typeof gid === 'string' ? gid : null
      } catch (e) {
        tr.dlState = 'error'
        this.host.notify(`Failed to start download: ${String(e)}`, 'error')
        return null
      }
    },
    async downloadAll() {
      if (this.downloadingAll) return
      this.downloadingAll = true
      this.cancelRequested = false
      // One group for the whole batch, named after the playlist.
      let groupId = ''
      try {
        const g = this.host.createGroup(this.playlistName || 'Spotify playlist')
        groupId = g && g.id ? g.id : ''
        if (groupId) this.host.notify(`Group created: ${g.name}`, 'info')
      } catch (e) {
        this.host.notify(`Could not create group: ${String(e)}`, 'error')
      }
      try {
        for (let i = 0; i < this.tracks.length; i++) {
          if (this.cancelRequested) break
          const tr = this.tracks[i]
          if (!tr.match || tr.dlState === 'done') continue
          const gid = await this.downloadTrack(i)
          if (this.cancelRequested) break
          if (gid && groupId) {
            try {
              await this.host.addToGroup(groupId, gid)
            } catch {
              /* grouping is best-effort; the download itself queued fine */
            }
          }
          await sleep(1200)
        }
      } finally {
        const stopped = this.cancelRequested
        this.downloadingAll = false
        this.cancelRequested = false
        if (stopped) {
          const left = this.tracks.filter((x) => x.match && x.dlState !== 'done').length
          this.host.notify(`Queueing stopped (${left} remaining). Started downloads keep running.`, 'info')
        }
      }
    },
    cancelQueue() {
      this.cancelRequested = true
    },
    // Clear everything: stop matching + queueing, cancel downloads this
    // session started that are still running, empty the list for a new search.
    // Finished downloads are never touched.
    async clearAll() {
      this.matchCancel = true
      this.cancelRequested = true
      const gids = [...new Set(this.tracks.map((t) => t.gid).filter((g) => typeof g === 'string' && g))]
      this.tracks = []
      this.playlistName = ''
      this.url = ''
      this.resolveError = ''
      this.resolveNote = ''
      if (!gids.length) return
      let stopped = 0
      let skipped = 0
      try {
        const all = await this.host.invoke('get_all_downloads')
        const active = new Set(
          (Array.isArray(all) ? all : [])
            .filter((d) => d && ['active', 'waiting', 'paused'].includes(d.status))
            .map((d) => d.gid),
        )
        for (const gid of gids) {
          if (!active.has(gid)) {
            skipped += 1
            continue
          }
          try {
            await this.host.cancelDownload(gid)
            stopped += 1
          } catch {
            skipped += 1
          }
        }
        this.host.notify(
          `List cleared. Stopped ${stopped} running download(s)${skipped ? `, ${skipped} already finished` : ''}.`,
          stopped ? 'success' : 'info',
        )
      } catch (e) {
        this.host.notify(`List cleared. Could not check running downloads: ${String(e)}`, 'info')
      }
    },
    reqStrip(h) {
      const st = this.reqStatus
      const items = [
        { key: 'ytdlp', label: 'yt-dlp', detail: '' },
        {
          key: 'runtime',
          label: (st && st.runtimeName) || 'JS runtime',
          detail: st && st.runtimeName ? `In use: ${st.runtimeName}` : 'Set up on the Media page',
        },
        {
          key: 'merger',
          label: (st && st.mergerName) || 'Merger (MP3)',
          detail: st && st.mergerName
            ? `In use: ${st.mergerName}`
            : 'Required for MP3 extraction — set up on the Media page',
        },
      ]
      return h('div', { style: 'display:flex;gap:6px;flex-wrap:wrap;align-items:center' },
        items.map((it) => {
          const ok = st ? !!st[it.key] : null
          const color = ok === null ? 'var(--text-dim)' : ok ? 'var(--accent-strong)' : '#e5484d'
          const icon = this.reqChecking ? 'hourglass_top' : ok === null ? 'help_outline' : ok ? 'check_circle' : 'error_outline'
          return h('span', {
            key: it.key,
            title: it.detail || 'Set up on the Media page',
            style: `display:inline-flex;align-items:center;gap:4px;font-size:11px;font-weight:600;color:${color};background:var(--hover-bg);border:1px solid var(--border);border-radius:100px;padding:4px 10px;max-width:190px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap`,
          }, [h('i', { class: 'material-icons', style: 'font-size:14px' }, icon), it.label])
        }).concat(
          st && (!st.ytdlp || !st.runtime || !st.merger)
            ? [h('span', { style: 'font-size:11px;color:var(--text-dim)' }, 'Missing pieces are set up on the Media page.')]
            : [],
        ),
      )
    },
    queueRing(h) {      const pct = Math.max(0, Math.min(100, this.queuePct))
      return h('svg', { width: 40, height: 40, viewBox: '0 0 42 42', title: `${pct}%` }, [
        h('circle', { cx: 21, cy: 21, r: 15.9155, fill: 'transparent', stroke: 'var(--border)', 'stroke-width': 4 }),
        h('circle', {
          cx: 21, cy: 21, r: 15.9155, fill: 'transparent',
          stroke: 'var(--accent-strong)', 'stroke-width': 4,
          'stroke-linecap': 'round',
          'stroke-dasharray': `${pct} 100`,
          transform: 'rotate(-90 21 21)',
          style: 'transition:stroke-dasharray 0.3s ease',
        }),
        h('text', {
          x: 21, y: 25, 'text-anchor': 'middle',
          fill: 'var(--text)', 'font-size': 9, 'font-weight': 700,
        }, `${pct}%`),
      ])
    },
  },
  render() {
    const { h } = this.host
    const card = (title, subtitle, icon, body) => h('section', { class: 'nd-card' }, [
      h('header', { class: 'nd-card-header' }, [
        h('div', { class: 'nd-card-head-left' }, [
          h('div', { class: 'nd-avatar' }, [h('i', { class: 'material-icons' }, icon)]),
          h('div', { class: 'nd-titles' }, [
            h('h2', { class: 'nd-title' }, title),
            h('span', { class: 'nd-subtitle' }, subtitle),
          ]),
        ]),
      ]),
      h('div', { class: 'nd-divider' }),
      h('div', { class: 'nd-card-body' }, body),
    ])

    const main = card('moonhunt-Spotify-playlist-bridge', 'v0.0.3 · Paste a Spotify link, match on YouTube, download.', 'library_music', [
      this.reqStrip(h),
      h('div', { class: 'nd-field' }, [
        h('label', { class: 'nd-label' }, [h('i', { class: 'material-icons nd-label-icon' }, 'link'), 'Spotify URL']),
        h('div', { class: 'nd-input-wrap' }, [
          h('input', {
            class: 'nd-input',
            placeholder: 'https://open.spotify.com/track/... or .../playlist/...',
            value: this.url,
            onInput: (e) => { this.url = e.target.value },
            onKeyup: (e) => { if (e.key === 'Enter') this.resolve() },
          }),
          this.url ? h('button', {
            class: 'nd-icon-btn nd-input-clear',
            title: 'Clear',
            onClick: () => { this.url = '' },
          }, [h('i', { class: 'material-icons' }, 'close')]) : null,
        ]),
      ]),
      h('div', { style: 'display:flex;align-items:center;gap:8px;flex-wrap:wrap' }, [
        h('button', {
          class: 'nd-btn nd-btn-primary',
          style: 'flex:0 1 auto',
          disabled: this.resolving || !this.url.trim(),
          onClick: () => this.resolve(),
        }, [h('i', { class: 'material-icons' }, this.resolving ? 'hourglass_top' : 'search'), this.resolving ? 'Resolving...' : 'Resolve tracks']),
        h('div', { style: 'display:flex;border:1px solid var(--border-strong);border-radius:8px;overflow:hidden' },
          ['mp3', 'mp4'].map((f) => h('button', {
            key: f,
            type: 'button',
            style: `border:none;background:${this.audioFormat === f ? 'rgba(var(--accent-rgb),0.12)' : 'var(--hover-bg)'};color:${this.audioFormat === f ? 'var(--accent-strong)' : 'var(--text-dim)'};font-size:11px;font-weight:700;padding:9px 12px;cursor:pointer;font-family:inherit`,
            onClick: () => { this.audioFormat = f },
          }, f.toUpperCase()))),
        this.downloadingAll
          ? h('div', { style: 'display:flex;align-items:center;gap:8px' }, [
              this.queueRing(h),
              h('span', { style: 'font-size:12px;color:var(--text-dim)' }, `Queuing ${this.queuedCount}/${this.queueTotal}`),
              h('button', {
                class: 'nd-btn nd-btn-secondary',
                style: 'flex:0 1 auto',
                onClick: () => this.cancelQueue(),
              }, [h('i', { class: 'material-icons' }, 'close'), 'Cancel']),
            ])
          : h('button', {
              class: 'nd-btn nd-btn-secondary',
              style: 'flex:0 1 auto',
              disabled: !this.readyCount,
              onClick: () => this.downloadAll(),
            }, [h('i', { class: 'material-icons' }, 'download'), `Download all (${this.readyCount})`]),
      ]),
      this.resolveError ? h('div', { class: 'spm-box spm-err' }, [
        h('i', { class: 'material-icons' }, 'error'), h('span', this.resolveError),
      ]) : null,
      this.resolveNote ? h('div', { class: 'spm-box spm-ok' }, [
        h('i', { class: 'material-icons' }, 'info'), h('span', this.resolveNote),
      ]) : null,
      h('div', { class: 'nd-field' }, [
        h('label', { class: 'nd-label' }, [h('i', { class: 'material-icons nd-label-icon' }, 'speed'), 'Search mode']),
        h('div', { class: 'nd-tabs seg4', role: 'tablist' }, [
          h('span', { class: `nd-tab-indicator pos${SEARCH_MODES.findIndex((m) => m.key === this.searchMode)}` }),
          ...SEARCH_MODES.map((m) => h('button', {
            key: m.key,
            role: 'tab',
            class: `nd-tab${this.searchMode === m.key ? ' active' : ''}`,
            'aria-selected': this.searchMode === m.key,
            title: (m.key === 'legacy' ? 'Legacy (Recommended): ' : '') + m.hint,
            onClick: () => this.setSearchMode(m.key),
          }, m.label)),
        ]),
        h('p', { class: 'nd-field-hint' }, (SEARCH_MODES.find((m) => m.key === this.searchMode) || SEARCH_MODES[0]).hint),
      ]),
      h('div', { class: 'nd-field' }, [
        h('button', {
          style: 'display:inline-flex;align-items:center;gap:4px;background:none;border:none;color:var(--text-dim);font-size:12px;cursor:pointer;font-family:inherit;padding:0',
          onClick: () => { this.showManual = !this.showManual },
        }, [h('i', { class: 'material-icons' }, this.showManual ? 'expand_less' : 'expand_more'), 'Or paste track names manually']),
        this.showManual ? h('textarea', {
          class: 'nd-input',
          style: 'height:auto;min-height:88px;padding:8px 10px;resize:vertical',
          rows: 4,
          placeholder: 'Artist - Title (one per line)',
          value: this.manualText,
          onInput: (e) => { this.manualText = e.target.value },
        }) : null,
        this.showManual ? h('button', {
          class: 'nd-btn nd-btn-secondary',
          style: 'flex:0 1 auto;align-self:flex-start',
          disabled: !this.manualText.trim(),
          onClick: () => this.useManualList(),
        }, [h('i', { class: 'material-icons' }, 'playlist_add'), 'Use list']) : null,
      ]),
    ])

    const list = this.tracks.length ? h('section', { class: 'nd-card' }, [
      h('header', { class: 'nd-card-header' }, [
        h('div', { class: 'nd-card-head-left' }, [
          h('div', { class: 'nd-titles' }, [
            h('h2', { class: 'nd-title' }, `Tracks (${this.tracks.length})`),
            h('span', { class: 'nd-subtitle' }, `${this.matchedCount} / ${this.tracks.length} matched`),
          ]),
        ]),
        h('div', { style: 'display:flex;gap:8px;flex-shrink:0' }, [
          this.matching
            ? h('button', {
                class: 'nd-btn nd-btn-secondary',
                style: 'flex:0 1 auto',
                onClick: () => this.cancelSearch(),
              }, [h('i', { class: 'material-icons' }, 'close'), 'Cancel'])
            : h('button', {
                class: 'nd-btn nd-btn-secondary',
                style: 'flex:0 1 auto',
                disabled: !this.unmatchedCount,
                onClick: () => this.matchAll(),
              }, [h('i', { class: 'material-icons' }, 'youtube_searched_for'), `Match all (${this.unmatchedCount})`]),
          h('button', {
            class: 'nd-btn nd-btn-secondary',
            style: 'flex:0 1 auto',
            disabled: !this.tracks.length && !this.matching,
            title: 'Clear list, stop matching/queueing and cancel still-running downloads from this list',
            onClick: () => this.clearAll(),
          }, [h('i', { class: 'material-icons' }, 'restart_alt'), 'Clear']),
        ]),
      ]),
      h('div', { class: 'nd-divider' }),
      h('ul', { style: 'list-style:none;margin:0;padding:8px 16px 16px;display:flex;flex-direction:column;gap:6px' },
        this.tracks.map((tr, i) => h('li', {
          key: i,
          style: 'display:flex;align-items:center;gap:10px;padding:8px 10px;border:1px solid var(--border);border-radius:10px',
        }, [
          h('span', { style: 'color:var(--text-dim);font-size:12px;min-width:20px;text-align:center' }, String(i + 1)),
          h('div', { style: 'flex:1;min-width:0' }, [
            h('div', { style: 'font-size:13px;font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap' }, tr.title),
            h('div', { style: 'font-size:11px;color:var(--text-dim)' }, tr.artist),
            tr.match
              ? h('div', { style: 'display:flex;align-items:center;gap:4px;font-size:11px;color:var(--accent-strong);margin-top:2px' }, [
                  h('i', { class: 'material-icons', style: 'font-size:14px' }, 'check_circle'),
                  h('span', { title: tr.match.webpage_url, style: 'overflow:hidden;text-overflow:ellipsis;white-space:nowrap' }, tr.match.title),
                  tr.match.duration ? h('small', `· ${fmtDur(tr.match.duration)}`) : null,
                ])
              : tr.matchState === 'searching'
                ? h('div', { style: 'display:flex;align-items:center;gap:6px;font-size:11px;color:var(--text-dim);margin-top:2px' }, [
                    h('span', { class: 'media-spinner' }), 'Searching YouTube...',
                  ])
                : tr.matchState === 'fail'
                  ? h('div', { title: tr.matchError || '', style: 'display:flex;align-items:center;gap:4px;font-size:11px;color:#e5484d;margin-top:2px' }, [
                      h('i', { class: 'material-icons', style: 'font-size:14px' }, 'warning'),
                      h('span', { style: 'overflow:hidden;text-overflow:ellipsis;white-space:nowrap' }, tr.matchError ? `No match (${tr.matchError})` : 'No match found'),
                    ])
                  : null,
          ]),
          h('div', { style: 'flex-shrink:0' }, [
            !tr.match
              ? h('button', {
                  class: 'nd-btn nd-btn-secondary',
                  style: 'flex:0 1 auto;padding:7px 10px',
                  disabled: tr.matchState === 'searching',
                  onClick: () => this.matchOne(i),
                }, [h('i', { class: 'material-icons' }, 'search')])
              : h('button', {
                  class: 'nd-btn nd-btn-primary',
                  style: 'flex:0 1 auto;padding:7px 10px',
                  disabled: tr.dlState === 'starting' || tr.dlState === 'done',
                  onClick: () => this.downloadTrack(i),
                }, [h('i', { class: 'material-icons' }, tr.dlState === 'done' ? 'check' : 'download')]),
          ]),
        ]))),
    ]) : null

    return h('div', { style: 'display:flex;flex-direction:column;gap:14px' }, [main, list])
  },
}
