//! Picker windows: the normal UI in "picker mode" (`index.html?picker=<id>`),
//! answering a file chooser request from another app (see `portal.rs`, which
//! receives those on Linux).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sonar_core::uipath;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

/// A file type filter: a name and (0 = glob, 1 = MIME type) patterns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub name: String,
    pub patterns: Vec<(u32, String)>,
}

/// Extra options the app asks along with the file: a checkbox (no options;
/// "true"/"false") or a choice from a list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub options: Vec<(String, String)>,
    pub selected: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerRequest {
    pub id: u64,
    /// Save dialog (a name to type) rather than Open.
    pub save: bool,
    pub title: String,
    pub accept_label: Option<String>,
    pub multiple: bool,
    /// Pick folders instead of files.
    pub directory: bool,
    pub filters: Vec<Filter>,
    /// Index into `filters`.
    pub current_filter: Option<usize>,
    pub choices: Vec<Choice>,
    #[serde(with = "uipath::opt")]
    pub folder: Option<PathBuf>,
    /// Suggested file name (Save), encoded like names in listings.
    pub name: Option<String>,
}

// Fields are read by the Linux portal glue only.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerAnswer {
    /// False when cancelled.
    pub ok: bool,
    #[serde(with = "uipath::vec")]
    pub paths: Vec<PathBuf>,
    pub choices: Vec<(String, String)>,
    pub filter: Option<usize>,
}

/// Shows picker windows; the Tauri side implements it (tests use a fake).
/// Only the Linux portal asks for them so far.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub trait Picker: Send + Sync + 'static {
    /// Show the request; answer through `reply` (dropping it cancels).
    fn show(&self, request: PickerRequest, reply: oneshot::Sender<PickerAnswer>);
    /// The app gave up on the request (closed its dialog): close the window.
    fn close(&self, id: u64);
}

pub const LABEL_PREFIX: &str = "picker-";

/// Requests waiting for their picker window to answer.
#[derive(Default)]
pub struct Pending(Mutex<HashMap<u64, (PickerRequest, oneshot::Sender<PickerAnswer>)>>);

impl Pending {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, (PickerRequest, oneshot::Sender<PickerAnswer>)>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub struct TauriPicker(pub AppHandle);

impl Picker for TauriPicker {
    fn show(&self, request: PickerRequest, reply: oneshot::Sender<PickerAnswer>) {
        let app = self.0.clone();
        let id = request.id;
        let title = if request.title.is_empty() {
            if request.save { "Save File".to_string() } else { "Open File".to_string() }
        } else {
            request.title.clone()
        };
        app.state::<Pending>().lock().insert(id, (request, reply));
        let label = format!("{LABEL_PREFIX}{id}");
        let built = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(format!("index.html?picker={id}").into()))
            .title(title)
            .inner_size(960.0, 620.0)
            .min_inner_size(560.0, 360.0)
            .center()
            .focused(true)
            .build();
        if built.is_err() {
            app.state::<Pending>().lock().remove(&id); // dropping the reply cancels
        }
    }

    fn close(&self, id: u64) {
        self.0.state::<Pending>().lock().remove(&id);
        if let Some(w) = self.0.get_webview_window(&format!("{LABEL_PREFIX}{id}")) {
            let _ = w.destroy();
        }
    }
}

/// A picker window closed: if it never answered, that's Cancel.
pub fn window_destroyed(app: &AppHandle, label: &str) {
    if let Some(id) = label.strip_prefix(LABEL_PREFIX).and_then(|n| n.parse::<u64>().ok()) {
        app.state::<Pending>().lock().remove(&id);
    }
}

#[tauri::command]
pub fn picker_request(pending: State<'_, Pending>, id: u64) -> Option<PickerRequest> {
    pending.lock().get(&id).map(|(r, _)| r.clone())
}

#[tauri::command]
pub fn picker_respond(app: AppHandle, pending: State<'_, Pending>, id: u64, answer: PickerAnswer) {
    if let Some((_, reply)) = pending.lock().remove(&id) {
        let _ = reply.send(answer);
    }
    if let Some(w) = app.get_webview_window(&format!("{LABEL_PREFIX}{id}")) {
        let _ = w.destroy();
    }
}

/// When a file chooser was last used. A Sonar that serves them keeps running
/// for a while after its last window closes, so the next one opens fast.
pub struct Usage {
    last: Arc<Mutex<Option<Instant>>>,
}

impl Usage {
    const IDLE: Duration = Duration::from_secs(5 * 60);

    /// `portal_only`: started just to serve a request, so count from now.
    pub fn new(portal_only: bool) -> Self {
        Self { last: Arc::new(Mutex::new(portal_only.then(Instant::now))) }
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn shared(&self) -> Arc<Mutex<Option<Instant>>> {
        self.last.clone()
    }

    /// Used recently enough to stay running without windows.
    pub fn keep_running(&self) -> bool {
        self.last.lock().unwrap_or_else(|p| p.into_inner()).is_some_and(|t| t.elapsed() < Self::IDLE)
    }
}
