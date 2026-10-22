mod host;
mod protocol;
mod worker;

use host::{Doc, Pool};
use protocol::{OutlineItem, PageSize, Request, SandboxStatus, ScanReport, TextLine};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Manager, State};

const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct AppState {
    pool: Arc<Pool>,
    launch: Mutex<Option<String>>,
}

#[derive(Serialize, Clone)]
struct DocInfo {
    id: u64,
    name: String,
    path: String,
    size: u64,
    sha256: String,
    pages: Vec<PageSize>,
    render_error: Option<String>,
    needs_password: bool,
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn display_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// Reads the file, or returns `None` if it holds more than `MAX_FILE_BYTES`.
/// The cap applies to the read itself because the file can grow after its size was checked.
fn read_capped(path: &str, size_hint: u64) -> std::io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::with_capacity(size_hint.min(MAX_FILE_BYTES) as usize);
    std::fs::File::open(path)?.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    Ok((bytes.len() as u64 <= MAX_FILE_BYTES).then_some(bytes))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

fn current_doc(pool: &Pool, id: u64) -> Result<Arc<Doc>, String> {
    match pool.current() {
        Some(d) if d.id == id => Ok(d),
        Some(_) => Err("That document is no longer open.".into()),
        None => Err("No document is open.".into()),
    }
}

#[tauri::command]
async fn open_document(app: AppHandle, path: String) -> Result<DocInfo, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        let meta = std::fs::metadata(&path).map_err(|e| format!("Couldn't read the file: {e}"))?;
        if !meta.is_file() {
            return Err("That path isn't a file.".into());
        }
        if meta.len() > MAX_FILE_BYTES {
            return Err("Files larger than 512 MB aren't supported.".into());
        }
        let bytes = read_capped(&path, meta.len())
            .map_err(|e| format!("Couldn't read the file: {e}"))?
            .ok_or("Files larger than 512 MB aren't supported.")?;
        let size = bytes.len() as u64;
        let sha256 = hex(&Sha256::digest(&bytes));
        let name = display_name(&path);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let doc = Arc::new(Doc { id, bytes: Arc::new(bytes), password: None });
        pool.set_document(Some(doc));
        let info = pool
            .call(0, Some(id), |w| Ok(w.loaded.as_ref().map(|(_, i)| i.clone()).unwrap_or_default()))
            .map_err(|e| e.to_string())?;
        Ok(DocInfo { id, name, path, size, sha256, pages: info.pages, render_error: info.render_error, needs_password: info.needs_password })
    })
    .await
}

#[derive(Serialize, Clone)]
struct Expanded {
    pdfs: Vec<String>,
    had_directory: bool,
    skipped: usize,
}

const MAX_BATCH_FILES: usize = 5000;

fn walk_pdfs(dir: &Path, depth: usize, visited: &mut HashSet<PathBuf>, out: &mut Vec<String>, skipped: &mut usize) {
    if depth > 16 || out.len() >= MAX_BATCH_FILES {
        return;
    }
    // Symlinks are followed, so each real directory is walked once to avoid loops.
    if let Ok(real) = std::fs::canonicalize(dir) {
        if !visited.insert(real) {
            return;
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        *skipped += 1;
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = std::fs::metadata(&path) else {
            *skipped += 1;
            continue;
        };
        if meta.is_dir() {
            walk_pdfs(&path, depth + 1, visited, out, skipped);
        } else if meta.is_file() && is_pdf(&path) {
            if out.len() >= MAX_BATCH_FILES {
                *skipped += 1;
            } else {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
}

fn is_pdf(path: &Path) -> bool {
    path.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false)
}

#[tauri::command]
async fn expand_paths(paths: Vec<String>) -> Result<Expanded, String> {
    blocking(move || {
        let mut pdfs = Vec::new();
        let mut had_directory = false;
        let mut skipped = 0;
        let mut visited = HashSet::new();
        for p in &paths {
            let path = Path::new(p);
            if path.is_dir() {
                had_directory = true;
                walk_pdfs(path, 0, &mut visited, &mut pdfs, &mut skipped);
            } else if path.is_file() {
                pdfs.push(p.clone());
            } else {
                skipped += 1;
            }
        }
        let mut seen = HashSet::new();
        pdfs.retain(|p| seen.insert(p.clone()));
        Ok(Expanded { pdfs, had_directory, skipped })
    })
    .await
}

#[derive(Serialize, Clone)]
struct FindingBrief {
    id: String,
    title: String,
    severity: String,
}

#[derive(Serialize, Clone)]
struct CheckResult {
    path: String,
    name: String,
    size: u64,
    pages: usize,
    sha256: String,
    verdict: String,
    findings: Vec<FindingBrief>,
    note: Option<String>,
    needs_password: bool,
    duration_ms: u64,
}

#[tauri::command]
async fn check_file(app: AppHandle, path: String) -> Result<CheckResult, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        let started = std::time::Instant::now();
        let meta = std::fs::metadata(&path).map_err(|e| format!("Couldn't read the file: {e}"))?;
        if !meta.is_file() {
            return Err("Not a file.".into());
        }
        if meta.len() > MAX_FILE_BYTES {
            return Err("Larger than 512 MB, skipped.".into());
        }
        let bytes = read_capped(&path, meta.len())
            .map_err(|e| format!("Couldn't read the file: {e}"))?
            .ok_or("Larger than 512 MB, skipped.")?;
        let sha256 = hex(&Sha256::digest(&bytes));
        let name = display_name(&path);
        let (info, report) = pool.check(&bytes).map_err(|e| e.to_string())?;
        Ok(CheckResult {
            path,
            name,
            size: bytes.len() as u64,
            pages: info.pages.len(),
            sha256,
            verdict: report.verdict.clone(),
            findings: report
                .findings
                .iter()
                .map(|f| FindingBrief { id: f.id.clone(), title: f.title.clone(), severity: f.severity.clone() })
                .collect(),
            note: info.render_error,
            needs_password: info.needs_password,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    })
    .await
}

#[tauri::command]
async fn finish_batch(app: AppHandle) -> Result<(), String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        pool.release_batch_worker();
        Ok(())
    })
    .await
}

#[tauri::command]
async fn unlock_document(app: AppHandle, id: u64, path: String, password: String) -> Result<DocInfo, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        let previous = current_doc(&pool, id)?;
        let new_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let doc = Arc::new(Doc { id: new_id, bytes: previous.bytes.clone(), password: Some(password) });
        pool.set_document(Some(doc));
        let loaded = pool
            .call(0, Some(new_id), |w| Ok(w.loaded.as_ref().map(|(_, i)| i.clone()).unwrap_or_default()))
            .map_err(|e| e.to_string())
            .and_then(|info| {
                if info.needs_password {
                    Err(info.render_error.unwrap_or_else(|| "That password didn't unlock the file.".into()))
                } else {
                    Ok(info)
                }
            });
        let info = match loaded {
            Ok(info) => info,
            Err(e) => {
                pool.replace_if_current(new_id, Some(previous));
                return Err(e);
            }
        };
        let name = display_name(&path);
        let size = previous.bytes.len() as u64;
        let sha256 = hex(&Sha256::digest(previous.bytes.as_slice()));
        Ok(DocInfo { id: new_id, name, path, size, sha256, pages: info.pages, render_error: info.render_error, needs_password: false })
    })
    .await
}

#[tauri::command]
async fn close_document(app: AppHandle, id: u64) -> Result<(), String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        if pool.current().map(|d| d.id) == Some(id) {
            pool.set_document(None);
            pool.release_workers();
        }
        Ok(())
    })
    .await
}

#[tauri::command]
async fn scan_document(app: AppHandle, id: u64) -> Result<ScanReport, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        current_doc(&pool, id)?;
        let reply = pool
            .call(1, Some(id), |w| w.request(&Request::Scan, &[], Duration::from_secs(180)))
            .map_err(|e| e.to_string())?;
        serde_json::from_value(reply.header.data).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn document_outline(app: AppHandle, id: u64) -> Result<Vec<OutlineItem>, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        current_doc(&pool, id)?;
        let reply = pool
            .call(1, Some(id), |w| w.request(&Request::Outline, &[], Duration::from_secs(60)))
            .map_err(|e| e.to_string())?;
        serde_json::from_value(reply.header.data).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn page_text(app: AppHandle, id: u64, page: usize) -> Result<Vec<TextLine>, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || {
        current_doc(&pool, id)?;
        let reply = pool
            .call(1, Some(id), |w| w.request(&Request::Text { page }, &[], Duration::from_secs(60)))
            .map_err(|e| e.to_string())?;
        serde_json::from_value(reply.header.data).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn sandbox_status(app: AppHandle) -> Result<SandboxStatus, String> {
    let pool = app.state::<AppState>().pool.clone();
    blocking(move || pool.sandbox_status().map_err(|e| e.to_string())).await
}

#[tauri::command]
fn dev_log(level: String, message: String) {
    match level.as_str() {
        "error" => log::error!(target: "webview", "{message}"),
        "warn" => log::warn!(target: "webview", "{message}"),
        _ => log::info!(target: "webview", "{message}"),
    }
}

#[cfg(debug_assertions)]
#[tauri::command]
fn dev_drive_poll() -> Vec<serde_json::Value> {
    let path = std::env::var("PAPER_WASP_DRIVE").unwrap_or_default();
    if path.is_empty() {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new() };
    if text.trim().is_empty() {
        return Vec::new();
    }
    let _ = std::fs::write(&path, "");
    text.lines()
        .filter_map(|l| {
            let mut parts = l.splitn(2, ' ');
            let cmd = parts.next()?.trim();
            if cmd.is_empty() {
                return None;
            }
            let arg = parts.next().map(|a| a.trim().to_string());
            Some(serde_json::json!({ "cmd": cmd, "arg": arg }))
        })
        .collect()
}

#[cfg(not(debug_assertions))]
#[tauri::command]
fn dev_drive_poll() -> Vec<serde_json::Value> {
    Vec::new()
}

#[tauri::command]
fn home_dir() -> Option<String> {
    ["HOME", "USERPROFILE"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok().filter(|h| !h.is_empty()))
}

#[tauri::command]
fn take_launch_file(state: State<'_, AppState>) -> Option<String> {
    state.launch.lock().ok().and_then(|mut l| l.take())
}

fn parse_query(query: Option<&str>) -> (f32, bool) {
    let mut scale = 1.0f32;
    let mut thumb = false;
    for pair in query.unwrap_or("").split('&') {
        let mut it = pair.splitn(2, '=');
        match (it.next(), it.next()) {
            (Some("s"), Some(v)) => scale = v.parse::<f32>().ok().filter(|v| v.is_finite()).unwrap_or(1.0),
            (Some("t"), Some(v)) => thumb = v == "1",
            _ => {}
        }
    }
    (scale, thumb)
}

fn serve_page(pool: &Pool, path: &str, query: Option<&str>) -> Result<Vec<u8>, (StatusCode, String)> {
    let mut parts = path.trim_start_matches('/').split('/');
    let (Some("page"), Some(id), Some(page)) = (parts.next(), parts.next(), parts.next()) else {
        return Err((StatusCode::NOT_FOUND, "unknown route".into()));
    };
    let id: u64 = id.parse().map_err(|_| (StatusCode::BAD_REQUEST, "bad document id".into()))?;
    let page: usize = page.parse().map_err(|_| (StatusCode::BAD_REQUEST, "bad page".into()))?;
    let (scale, thumb) = parse_query(query);
    current_doc(pool, id).map_err(|e| (StatusCode::NOT_FOUND, e))?;
    let reply = pool
        .call(if thumb { 1 } else { 0 }, Some(id), |w| w.request(&Request::Render { page, scale }, &[], Duration::from_secs(90)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(reply.payload)
}

pub fn run() {
    if std::env::args().any(|a| a == "--wasp-worker") {
        worker::run();
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let launch = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--") && Path::new(a).is_file())
        .map(|a| std::path::absolute(&a).map(|p| p.to_string_lossy().into_owned()).unwrap_or(a));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState { pool: Arc::new(Pool::new(2)), launch: Mutex::new(launch) })
        .setup(|app| {
            if let (Some(icon), Some(window)) = (app.default_window_icon().cloned(), app.get_webview_window("main")) {
                let _ = window.set_icon(icon);
            }
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("wasp", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let uri = request.uri().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let pool = app.state::<AppState>().pool.clone();
                log::debug!("wasp protocol request: {} ?{}", uri.path(), uri.query().unwrap_or(""));
                let response = match serve_page(&pool, uri.path(), uri.query()) {
                    Ok(png) => Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "image/png")
                        .header("Cache-Control", "no-store")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(png)
                        .unwrap(),
                    Err((status, msg)) => {
                        log::warn!("wasp protocol error {status}: {msg}");
                        Response::builder()
                        .status(status)
                        .header("Content-Type", "text/plain; charset=utf-8")
                        .body(msg.into_bytes())
                        .unwrap()
                    }
                };
                responder.respond(response);
            });
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            unlock_document,
            close_document,
            expand_paths,
            check_file,
            finish_batch,
            scan_document,
            document_outline,
            page_text,
            sandbox_status,
            take_launch_file,
            home_dir,
            dev_log,
            dev_drive_poll
        ])
        .run(tauri::generate_context!())
        .expect("error while running Paper Wasp");
}
