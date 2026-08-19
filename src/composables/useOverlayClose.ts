/**
 * Outside-click close guard for modal overlays.
 *
 * Replaces the naive `@click.self` close, which also fires when the user
 * presses inside the modal and RELEASES outside of it (or presses outside
 * and releases inside). With this guard the overlay only closes when both
 * the press AND the release happen on the overlay itself — dragging a click
 * out of / into the modal never closes it.
 *
 * Usage (template):
 *   <div class="overlay"
 *        @mousedown.self.prevent="overlayPress()"
 *        @mouseup.self="overlayRelease()"
 *        @mouseleave="overlayCancel()">
 */
export function useOverlayClose() {
  let pressedOnOverlay = false

  /** mousedown on the overlay (bubbling stopped by the modal's click.stop). */
  function overlayPress() {
    pressedOnOverlay = true
  }

  /** Pointer left the overlay while pressed — not a valid outside click. */
  function overlayCancel() {
    pressedOnOverlay = false
  }

  /** mouseup on the overlay. Closes only if the press started there too. */
  function overlayRelease(): boolean {
    if (!pressedOnOverlay) return false
    pressedOnOverlay = false
    return true
  }

  return { overlayPress, overlayCancel, overlayRelease }
}