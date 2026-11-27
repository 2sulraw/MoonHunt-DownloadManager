//! Screen modules.

pub mod addon;
pub mod spotify;
pub mod downloads;
pub mod home;
pub mod media;
pub mod placeholder;
pub mod settings;

use crate::state::UiState;

/// Backend execution context handed to screens: a live `AppHandle`, a shared
/// multi-thread tokio runtime to spawn core async calls on, the toast
/// queue action results land in, and the event inbox for payload fetches.
#[derive(Clone)]
pub struct Ctx {
    pub core: moonhunt_core::AppHandle,
    pub rt: std::sync::Arc<tokio::runtime::Runtime>,
    pub toasts: crate::state::ToastQueue,
    pub inbox: crate::Inbox,
    /// Async results land in queues the frame loop reads; without an explicit
    /// wake the app idles on the last frame and never shows them.
    pub egui_ctx: egui::Context,
}

impl Ctx {
    /// Run a core async call on the backend runtime; the result becomes a
    /// toast. `f` receives an owned `AppHandle` clone and returns a future.
    pub fn run<F>(&self, f: impl FnOnce(moonhunt_core::AppHandle) -> F + Send + 'static)
    where
        F: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        let core = self.core.clone();
        let toasts = self.toasts.clone();
        let egui_ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match f(core).await {
                // Empty Ok messages skip the toast (used for data fetches).
                Ok(msg) if !msg.is_empty() => Self::push(&toasts, true, msg),
                Err(e) => Self::push(&toasts, false, e),
                _ => {}
            }
            egui_ctx.request_repaint();
        });
    }

    /// Queue a toast for results the UI composes itself (staged flows like
    /// the settings save, or messages that don't come from a core call).
    pub fn toast(&self, ok: bool, text: impl Into<String>) {
        Self::push(&self.toasts, ok, text.into());
    }

    fn push(toasts: &crate::state::ToastQueue, ok: bool, text: String) {
        if let Ok(mut q) = toasts.lock() {
            q.push_back((ok, text));
        }
    }

    /// Run a core async call whose *payload* (not message) is needed by the
    /// UI: the result lands in the event inbox as `{ok, data|error}` under
    /// `name` and is applied by `events::apply` on the next drain.
    pub fn emit<F>(&self, name: &'static str, f: impl FnOnce(moonhunt_core::AppHandle) -> F + Send + 'static)
    where
        F: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        let egui_ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            let payload = match f(core).await {
                Ok(data) => serde_json::json!({"ok": true, "data": data}),
                Err(e) => serde_json::json!({"ok": false, "error": e}),
            };
            if let Ok(mut q) = inbox.lock() {
                q.push_back((name.to_string(), payload));
            }
            egui_ctx.request_repaint();
        });
    }
}

/// The CentralPanel keeps a 20px right margin for the page content; a page
/// scroller has to span the whole pane so its bar sits flush with the window
/// edge (the WebView puts the scrollbar at the edge and the gutter inside the
/// padding). Screens build their ScrollArea on this rect and take the gutter
/// back off the content.
pub const PAGE_GUTTER: f32 = 20.0;

pub fn page_viewport(ui: &egui::Ui) -> egui::Rect {
    let mut r = ui.available_rect_before_wrap();
    r.max.x += PAGE_GUTTER;
    r
}

/// Draws the currently active screen.
pub fn show(ui_state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui) {
    match ui_state.screen {
        crate::state::Screen::Home => home::show(ui_state, ctx, ui),
        crate::state::Screen::Downloads => downloads::show(ui_state, ctx, ui),
        crate::state::Screen::Media => media::show(ui_state, ctx, ui),
        crate::state::Screen::Addon => addon::show(ui_state, ctx, ui),
        crate::state::Screen::Settings => settings::show(ui_state, ctx, ui),
    }
}
