// App-wide hover tooltip. Any element carrying a native `title` (including
// reactive `:title` bindings) shows a styled, no-delay preview on hover.
//
// It uses ONE floating bubble appended to <body> rather than injecting a child
// into each target: injecting into Vue-owned DOM breaks on re-render (Vue drops
// the injected node), which is what killed the sidebar speed toggle. Text is
// also re-read when Vue re-applies a `title` attribute, so reactive tooltips
// stay live.
//
// Placement is computed on hover (flip above/below, clamp to viewport) and the
// bubble is position:fixed so clipped containers can't cut it off.

let initialised = false

const GAP = 8
let tip: HTMLElement | null = null
let current: Element | null = null

function ensureTip(): HTMLElement {
  if (tip) return tip
  tip = document.createElement('div')
  tip.className = 'ttp-note'
  document.body.appendChild(tip)
  return tip
}

function upgrade(el: Element) {
  if (el.hasAttribute('data-ttp')) return
  const title = el.getAttribute('title')
  if (!title) return
  el.setAttribute('data-ttp', '1')
  el.setAttribute('data-ttp-text', title)
  el.removeAttribute('title')
}

// Vue re-applied a reactive :title on a known host — refresh the live text.
function resync(el: Element) {
  const title = el.getAttribute('title')
  if (!title) return
  el.setAttribute('data-ttp-text', title)
  el.removeAttribute('title')
}

function walk(node: Node) {
  if (node.nodeType !== Node.ELEMENT_NODE) return
  const el = node as Element
  upgrade(el)
  el.querySelectorAll('[title]').forEach(upgrade)
}

function show(host: Element) {
  current = host
  const note = ensureTip()
  note.textContent = host.getAttribute('data-ttp-text') || ''

  const r = host.getBoundingClientRect()
  const w = note.offsetWidth
  const h = note.offsetHeight
  const vw = window.innerWidth
  const vh = window.innerHeight

  let left = r.left + r.width / 2 - w / 2
  left = Math.max(GAP, Math.min(left, vw - w - GAP))
  let top = r.top - h - GAP
  if (top < GAP) top = r.bottom + GAP
  top = Math.min(Math.max(top, GAP), vh - h - GAP)

  note.style.left = `${left}px`
  note.style.top = `${top}px`
  note.style.opacity = '1'
  note.style.visibility = 'visible'
}

function hide() {
  current = null
  if (!tip) return
  tip.style.opacity = '0'
  tip.style.visibility = 'hidden'
}

function isElement(node: unknown): node is Element {
  return node instanceof Element
}

function hostOf(target: unknown): Element | null {
  return isElement(target) ? target.closest('[data-ttp]') : null
}

function onOver(e: MouseEvent) {
  const host = hostOf(e.target)
  if (host) {
    // A reactive :title re-applied by Vue after the last observer tick can
    // still trigger the native browser tooltip. Strip it synchronously so only
    // the styled preview ever shows.
    const liveTitle = host.getAttribute('title')
    if (liveTitle) {
      host.setAttribute('data-ttp-text', liveTitle)
      host.removeAttribute('title')
    }
    show(host)
    return
  }
  if (current && !isElement(e.target)) hide()
}

function onOut(e: MouseEvent) {
  if (!current) return
  if (isElement(e.relatedTarget) && current.contains(e.relatedTarget)) return
  hide()
}

function onFocusIn(e: FocusEvent) {
  const host = hostOf(e.target)
  if (host) show(host)
}

function onFocusOut(e: FocusEvent) {
  if (!current) return
  if (isElement(e.relatedTarget) && current.contains(e.relatedTarget)) return
  hide()
}

export function initGlobalTooltips() {
  if (initialised) return
  initialised = true

  document.querySelectorAll('[title]').forEach(upgrade)

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'attributes') {
        resync(mutation.target as Element)
      } else if (mutation.type === 'childList') {
        for (const node of mutation.addedNodes) walk(node)
      }
    }
  })
  observer.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['title'],
  })

  document.addEventListener('mouseover', onOver, true)
  document.addEventListener('mouseout', onOut, true)
  document.addEventListener('focusin', onFocusIn, true)
  document.addEventListener('focusout', onFocusOut, true)
}