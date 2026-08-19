/**
 * SortableJS drag reorder with a whole-card grab surface (no in-card handle)
 * and a glide-settle drop animation.
 */
import { ref, onUnmounted, nextTick, type Ref } from 'vue'
import Sortable from 'sortablejs'
import type { SortableOptions, SortableEvent } from 'sortablejs'

/** Shared base drag options. */
export const SORTABLE_BASE: Partial<SortableOptions> = {
  animation: 240,
  filter: 'button, a, input, textarea, select, [data-no-drag]',
  ghostClass: 'drag-item--ghost',
  chosenClass: 'drag-item--chosen',
  fallbackClass: 'drag-item--floating',
  dragClass: 'drag-item--dragging',
  direction: 'vertical',
  swapThreshold: 0.72,
  invertedSwapThreshold: 0.28,
  invertSwap: false,
  forceFallback: true,
  fallbackOnBody: true,
  fallbackTolerance: 3,
  preventOnFilter: false,
}

export interface DragReorderReturn {
  grabEnabled: Ref<boolean>
  toggleGrabEnabled: () => void
  sorting: Ref<boolean>
  mountSortable: (el: HTMLElement, opts?: Partial<SortableOptions>) => Sortable | null
  destroyAllSortables: () => void
  startFloatingRectTracking: () => void
  animateDropSettle: (event: SortableEvent | undefined) => Promise<void>
  stopFloatingRectTracking: () => void
}

export interface DragReorderOptions {
  onReorder?: (oldIndex: number, newIndex: number) => void
  direction?: 'vertical' | 'horizontal' | null
}

export function useDragReorder(options: DragReorderOptions): DragReorderReturn {
  const { onReorder = () => {}, direction = 'vertical' } = options

  const grabEnabled = ref(false)
  const sorting = ref(false)

  let sortables: Sortable[] = []
  let lastFloatingRect: DOMRect | null = null
  let floatingRectFrame = 0

  function toggleGrabEnabled() {
    grabEnabled.value = !grabEnabled.value
  }

  function trackFloatingRect() {
    const floating = document.querySelector(
      '.drag-item--floating, .group-container--floating',
    ) as HTMLElement | null
    if (floating?.isConnected) {
      lastFloatingRect = floating.getBoundingClientRect()
    }
    if (sorting.value) {
      floatingRectFrame = requestAnimationFrame(trackFloatingRect)
    }
  }

  function startFloatingRectTracking() {
    stopFloatingRectTracking()
    lastFloatingRect = null
    floatingRectFrame = requestAnimationFrame(trackFloatingRect)
  }

  function stopFloatingRectTracking() {
    if (!floatingRectFrame) return
    cancelAnimationFrame(floatingRectFrame)
    floatingRectFrame = 0
  }

  /** Glide the floating drag rect into its final slot. */
  function animateDropSettle(event: SortableEvent | undefined): Promise<void> {
    const item = event?.item
    if (!lastFloatingRect || !item?.isConnected) return Promise.resolve()

    const targetRect = item.getBoundingClientRect()
    const deltaX = lastFloatingRect.left - targetRect.left
    const deltaY = lastFloatingRect.top - targetRect.top
    lastFloatingRect = null

    if (Math.abs(deltaX) < 1 && Math.abs(deltaY) < 1) return Promise.resolve()

    item.classList.add('drag-item--settling')
    item.style.setProperty('--drag-drop-x', `${deltaX}px`)
    item.style.setProperty('--drag-drop-y', `${deltaY}px`)

    return new Promise((resolve) => {
      requestAnimationFrame(() => {
        item.classList.add('drag-item--settled')
      })

      window.setTimeout(() => {
        item.classList.remove('drag-item--settling', 'drag-item--settled')
        item.style.removeProperty('--drag-drop-x')
        item.style.removeProperty('--drag-drop-y')
        resolve()
      }, 320)
    })
  }

  function destroyAllSortables() {
    sortables.forEach((s) => {
      try {
        s.destroy()
      } catch {
        /* already destroyed */
      }
    })
    sortables = []
    stopFloatingRectTracking()
    lastFloatingRect = null
  }

  /**
   * Create one Sortable on `el`. Caller must destroy first when remounting.
   * Whole card is draggable (no handle); MoonHunt uses grab-mode instead.
   */
  function mountSortable(el: HTMLElement, extraOpts?: Partial<SortableOptions>): Sortable | null {
    if (!grabEnabled.value) return null

    // Allow extraOpts to fully override lifecycle hooks when provided.
    const userOnStart = extraOpts?.onStart
    const userOnUpdate = extraOpts?.onUpdate
    const userOnEnd = extraOpts?.onEnd

    const { onStart: _s, onUpdate: _u, onEnd: _e, ...restExtra } = extraOpts ?? {}

    const sortableOpts: SortableOptions = {
      ...SORTABLE_BASE,
      draggable: '.drag-item',
      direction: direction ?? undefined,
      ...restExtra,
      // No `handle` — entire .drag-item is the grab surface (when grab enabled)
      onStart: (event: SortableEvent) => {
        sorting.value = true
        startFloatingRectTracking()
        userOnStart?.(event)
      },
      onUpdate: (event: SortableEvent) => {
        if (userOnUpdate) {
          userOnUpdate(event)
          return
        }
        if (
          event.oldIndex === undefined ||
          event.newIndex === undefined ||
          event.oldIndex === event.newIndex
        ) {
          return
        }
        onReorder(event.oldIndex, event.newIndex)
      },
      onEnd: async (event: SortableEvent) => {
        stopFloatingRectTracking()
        // Settle after the DOM updates, then clear the sorting flag
        await nextTick()
        await animateDropSettle(event)
        if (userOnEnd) {
          await userOnEnd(event)
        }
        window.setTimeout(() => {
          sorting.value = false
        }, 0)
      },
    }

    const instance = Sortable.create(el, sortableOpts)
    sortables.push(instance)
    return instance
  }

  onUnmounted(() => {
    destroyAllSortables()
  })

  return {
    grabEnabled,
    toggleGrabEnabled,
    sorting,
    mountSortable,
    destroyAllSortables,
    startFloatingRectTracking,
    animateDropSettle,
    stopFloatingRectTracking,
  }
}
