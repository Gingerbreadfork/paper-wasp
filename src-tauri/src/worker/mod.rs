//! The sandboxed worker: reads a document over stdin, answers requests over stdout.

pub mod render;
pub mod sandbox;
pub mod scan;
pub mod text;

use crate::protocol::{LoadInfo, PageSize, ReplyHeader, Request};
use hayro::hayro_interpret::{InterpreterCache, InterpreterSettings};
use hayro::hayro_syntax::{DecryptionError, LoadPdfError, Pdf};
use hayro::RenderCache;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

type Input = io::BufReader<io::StdinLock<'static>>;
type Output = io::BufWriter<io::StdoutLock<'static>>;

fn next_request(input: &mut Input) -> Option<(Request, Vec<u8>)> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if line.trim().is_empty() {
            continue;
        }
        let req: Request = serde_json::from_str(line.trim_end()).ok()?;
        let mut payload = Vec::new();
        if let Request::Load { len, .. } = req {
            payload.resize(len, 0);
            input.read_exact(&mut payload).ok()?;
        }
        return Some((req, payload));
    }
}

fn send(out: &mut Output, header: ReplyHeader, payload: &[u8]) {
    let mut h = header;
    h.len = payload.len();
    let bytes = serde_json::to_vec(&h).unwrap_or_default();
    let _ = out.write_all(&bytes);
    let _ = out.write_all(b"\n");
    let _ = out.write_all(payload);
    let _ = out.flush();
}

fn ok(data: Value) -> ReplyHeader {
    ReplyHeader { ok: true, error: None, len: 0, data }
}

fn err(msg: impl Into<String>) -> ReplyHeader {
    ReplyHeader { ok: false, error: Some(msg.into()), len: 0, data: Value::Null }
}

struct Parsed {
    doc: Option<lopdf::Document>,
    locked: bool,
    encrypted: bool,
}

fn parse_lopdf(data: &[u8], password: &str) -> Parsed {
    let loaded = catch_unwind(|| {
        if password.is_empty() {
            lopdf::Document::load_mem(data)
        } else {
            let options = lopdf::LoadOptions { password: Some(password.to_string()), ..Default::default() };
            lopdf::Document::load_mem_with_options(data, options).or_else(|_| lopdf::Document::load_mem(data))
        }
    });
    let Ok(Ok(mut doc)) = loaded else {
        return Parsed { doc: None, locked: false, encrypted: false };
    };
    let encrypted = doc.is_encrypted() || doc.trailer.has(b"Encrypt");
    let mut locked = false;
    if doc.is_encrypted() {
        let mut unlocked = catch_unwind(AssertUnwindSafe(|| doc.decrypt(password).is_ok())).unwrap_or(false);
        if !unlocked && !password.is_empty() {
            unlocked = catch_unwind(AssertUnwindSafe(|| doc.decrypt("").is_ok())).unwrap_or(false);
        }
        locked = !unlocked;
    }
    Parsed { doc: Some(doc), locked, encrypted }
}

pub fn run() -> ! {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[wasp-worker] panic: {info}");
    }));
    let status = sandbox::lock_down();
    let status_json = serde_json::to_value(&status).unwrap_or(Value::Null);

    let stdin = Box::leak(Box::new(io::stdin()));
    let stdout = Box::leak(Box::new(io::stdout()));
    let mut input: Input = io::BufReader::with_capacity(1 << 20, stdin.lock());
    let mut output: Output = io::BufWriter::with_capacity(1 << 20, stdout.lock());

    let mut pending: Option<(Vec<u8>, Option<String>)> = None;
    loop {
        let (bytes, password) = match pending.take() {
            Some(b) => b,
            None => match next_request(&mut input) {
                None => std::process::exit(0),
                Some((Request::Load { password, .. }, payload)) => (payload, password),
                Some((Request::Status, _)) => {
                    send(&mut output, ok(status_json.clone()), &[]);
                    continue;
                }
                Some((Request::Unload, _)) => {
                    send(&mut output, ok(Value::Null), &[]);
                    continue;
                }
                Some(_) => {
                    send(&mut output, err("no document is loaded"), &[]);
                    continue;
                }
            },
        };

        let data = Arc::new(bytes);
        let password = password.unwrap_or_default();
        let parsed = catch_unwind({
            let data = data.clone();
            let password = password.clone();
            move || Pdf::new_with_password(data, &password)
        });
        let mut needs_password = false;
        let (pdf, render_error) = match parsed {
            Ok(Ok(p)) => (Some(p), None),
            Ok(Err(LoadPdfError::Decryption(DecryptionError::PasswordProtected))) => {
                needs_password = true;
                let msg = if password.is_empty() {
                    "This file is password-protected."
                } else {
                    "That password didn't unlock the file."
                };
                (None, Some(msg.to_string()))
            }
            Ok(Err(LoadPdfError::Decryption(DecryptionError::UnsupportedAlgorithm))) => {
                (None, Some("This file uses an encryption method the renderer doesn't support.".to_string()))
            }
            Ok(Err(e)) => (None, Some(format!("The renderer couldn't parse this file ({e:?})."))),
            Err(_) => (None, Some("The renderer crashed while parsing this file.".to_string())),
        };
        let pages: Vec<PageSize> = pdf
            .as_ref()
            .map(|p| {
                p.pages()
                    .iter()
                    .map(|pg| {
                        let (width, height) = pg.render_dimensions();
                        PageSize { width, height }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let info = LoadInfo { pages, version: String::new(), render_error, needs_password };
        send(&mut output, ok(serde_json::to_value(&info).unwrap_or(Value::Null)), &[]);

        let cache = RenderCache::new();
        let text_cache = InterpreterCache::new();
        let settings = InterpreterSettings::default();
        let mut structure: Option<Parsed> = None;

        loop {
            let Some((req, payload)) = next_request(&mut input) else {
                std::process::exit(0);
            };
            match req {
                Request::Load { password, .. } => {
                    pending = Some((payload, password));
                    break;
                }
                Request::Unload => {
                    send(&mut output, ok(Value::Null), &[]);
                    break;
                }
                Request::Status => send(&mut output, ok(status_json.clone()), &[]),
                Request::Render { page, scale } => match &pdf {
                    Some(p) => {
                        let result = catch_unwind(AssertUnwindSafe(|| render::render_png(p, &cache, &settings, page, scale)));
                        match result {
                            Ok(Ok((w, h, png))) => send(&mut output, ok(json!({ "width": w, "height": h })), &png),
                            Ok(Err(e)) => send(&mut output, err(e), &[]),
                            Err(_) => send(&mut output, err("The renderer crashed on this page."), &[]),
                        }
                    }
                    None => send(&mut output, err("This file couldn't be parsed for rendering."), &[]),
                },
                Request::Text { page } => match &pdf {
                    Some(p) => {
                        let result = catch_unwind(AssertUnwindSafe(|| text::page_text(p, &text_cache, &settings, page)));
                        match result {
                            Ok(Ok(lines)) => send(&mut output, ok(serde_json::to_value(&lines).unwrap_or(Value::Null)), &[]),
                            Ok(Err(e)) => send(&mut output, err(e), &[]),
                            Err(_) => send(&mut output, err("Text extraction crashed on this page."), &[]),
                        }
                    }
                    None => send(&mut output, err("This file couldn't be parsed."), &[]),
                },
                Request::Scan => {
                    let parsed = structure.get_or_insert_with(|| parse_lopdf(&data, &password));
                    let (locked, encrypted) = (parsed.locked || needs_password, parsed.encrypted);
                    let page_count = info.pages.len();
                    let result = catch_unwind(AssertUnwindSafe(|| scan::scan(&data, parsed.doc.as_ref(), page_count, locked, encrypted)));
                    match result {
                        Ok(report) => send(&mut output, ok(serde_json::to_value(&report).unwrap_or(Value::Null)), &[]),
                        Err(_) => {
                            let fallback = catch_unwind(AssertUnwindSafe(|| scan::scan(&data, None, page_count, locked, encrypted)));
                            match fallback {
                                Ok(report) => send(&mut output, ok(serde_json::to_value(&report).unwrap_or(Value::Null)), &[]),
                                Err(_) => send(&mut output, err("The scanner crashed on this file."), &[]),
                            }
                        }
                    }
                }
                Request::Outline => {
                    let parsed = structure.get_or_insert_with(|| parse_lopdf(&data, &password));
                    let items = match parsed.doc.as_ref() {
                        Some(d) => catch_unwind(AssertUnwindSafe(|| scan::outline(d))).unwrap_or_default(),
                        None => Vec::new(),
                    };
                    send(&mut output, ok(serde_json::to_value(&items).unwrap_or(Value::Null)), &[]);
                }
            }
        }
    }
}
