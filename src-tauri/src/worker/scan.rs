//! Inspects a PDF for active content, at the byte level and through its object graph.

use crate::protocol::{Finding, Link, Metadata, OutlineItem, ScanReport};
use lopdf::{Dictionary, Document, Object, ObjectId};
use memchr::{memchr_iter, memmem};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

const WATCH: &[&str] = &[
    "JS", "JavaScript", "AA", "OpenAction", "Launch", "URI", "SubmitForm", "GoToR", "GoToE",
    "ImportData", "EmbeddedFile", "EmbeddedFiles", "FileAttachment", "RichMedia", "XFA",
    "AcroForm", "Encrypt", "ObjStm", "JBIG2Decode", "Sound", "Movie", "3D", "Rendition",
    "Screen", "Page", "Annots", "Link", "Widget", "Sig", "ASCIIHexDecode", "ASCII85Decode",
];

const MAX_DEPTH: usize = 24;
const SNIPPET: usize = 160;

#[derive(Default)]
struct Raw {
    counts: BTreeMap<String, usize>,
    obfuscated: BTreeMap<String, usize>,
    header_offset: Option<usize>,
    version: String,
    eof_count: usize,
    trailing: usize,
    objects: usize,
    streams: usize,
    polyglot: Vec<String>,
}

fn is_regular(b: u8) -> bool {
    !matches!(
        b,
        b'\0' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'(' | b')' | b'<' | b'>' | b'[' | b']'
            | b'{' | b'}' | b'/' | b'%'
    )
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn raw_scan(data: &[u8]) -> Raw {
    let mut raw = Raw::default();
    let watch: HashSet<&str> = WATCH.iter().copied().collect();

    for pos in memchr_iter(b'/', data) {
        let mut name = Vec::with_capacity(16);
        let mut i = pos + 1;
        let mut escaped = false;
        while i < data.len() && is_regular(data[i]) && name.len() < 48 {
            if data[i] == b'#' && i + 2 < data.len() {
                if let (Some(h), Some(l)) = (hex_val(data[i + 1]), hex_val(data[i + 2])) {
                    let c = (h << 4) | l;
                    if c.is_ascii_alphanumeric() {
                        escaped = true;
                    }
                    name.push(c);
                    i += 3;
                    continue;
                }
            }
            name.push(data[i]);
            i += 1;
        }
        if let Ok(s) = std::str::from_utf8(&name) {
            if watch.contains(s) {
                *raw.counts.entry(s.to_string()).or_default() += 1;
                if escaped {
                    *raw.obfuscated.entry(s.to_string()).or_default() += 1;
                }
            }
        }
    }

    raw.objects = memmem::find_iter(data, b"endobj").count();
    raw.streams = memmem::find_iter(data, b"endstream").count();
    raw.eof_count = memmem::find_iter(data, b"%%EOF").count();

    let head = &data[..data.len().min(1024)];
    raw.header_offset = memmem::find(head, b"%PDF-");
    if let Some(off) = raw.header_offset {
        let v: String = data[off + 5..]
            .iter()
            .take(8)
            .take_while(|b| b.is_ascii_digit() || **b == b'.')
            .map(|b| *b as char)
            .collect();
        raw.version = v;
    }
    if let Some(last) = memmem::rfind(data, b"%%EOF") {
        let tail = &data[last + 5..];
        let non_ws = tail.iter().filter(|b| !b.is_ascii_whitespace() && **b != 0).count();
        raw.trailing = non_ws;
    }

    if data.starts_with(b"MZ") {
        raw.polyglot.push("Windows executable header (MZ)".into());
    }
    if data.starts_with(b"\x7fELF") {
        raw.polyglot.push("Linux executable header (ELF)".into());
    }
    // ZIP readers look for the end-of-central-directory record only in the file's last 64 KiB + 22 bytes.
    let zip_tail = &data[data.len().saturating_sub(65_535 + 22)..];
    let zip_at_end = memmem::find(zip_tail, b"PK\x05\x06").is_some() && memmem::find(data, b"PK\x03\x04").is_some();
    if data.starts_with(b"PK\x03\x04") || zip_at_end {
        raw.polyglot.push("ZIP archive structure".into());
    }
    let lower_head: Vec<u8> = head.iter().map(|b| b.to_ascii_lowercase()).collect();
    if memmem::find(&lower_head, b"<html").is_some() || memmem::find(&lower_head, b"<!doctype").is_some() {
        raw.polyglot.push("HTML document header".into());
    }
    raw
}

#[derive(Default)]
struct Structure {
    js: usize,
    js_snippets: Vec<String>,
    js_text: String,
    aa_js: bool,
    launch: Vec<String>,
    open_action: Option<String>,
    open_dest_page: Option<usize>,
    aa: usize,
    submit: Vec<String>,
    import_data: usize,
    gotor: Vec<String>,
    attachments: Vec<String>,
    media: Vec<String>,
    xfa: bool,
    acroform: bool,
    widgets: usize,
    signatures: usize,
    links: Vec<Link>,
    uri_count: usize,
    objstm: usize,
    jbig2: usize,
    encrypted: bool,
    locked: bool,
}

fn clean_text(s: &str, limit: usize) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.chars().count() > limit {
        out = out.chars().take(limit).collect::<String>() + "…";
    }
    out
}

pub fn pdf_string(bytes: &[u8]) -> String {
    let s = if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else if let Ok(s) = std::str::from_utf8(bytes) {
        s.to_string()
    } else {
        bytes.iter().map(|b| *b as char).collect()
    };
    clean_text(&s, 400)
}

fn pdf_date(raw: &str) -> String {
    let t = raw.trim_start_matches("D:");
    let digits: Vec<char> = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return clean_text(raw, 40);
    }
    let s: String = digits.iter().collect();
    let mut out = format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]);
    if digits.len() >= 12 {
        out.push_str(&format!(" {}:{}", &s[8..10], &s[10..12]));
    }
    out
}

fn deref<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    doc.dereference(obj).ok().map(|(_, o)| o)
}

fn dict_of<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match deref(doc, obj)? {
        Object::Dictionary(d) => Some(d),
        Object::Stream(s) => Some(&s.dict),
        _ => None,
    }
}

fn name_of(obj: &Object) -> Option<String> {
    obj.as_name().ok().map(|n| String::from_utf8_lossy(n).into_owned())
}

fn string_of(doc: &Document, obj: &Object) -> Option<String> {
    match deref(doc, obj)? {
        Object::String(b, _) => Some(pdf_string(b)),
        Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
        _ => None,
    }
}

fn filespec_name(doc: &Document, obj: &Object) -> Option<String> {
    match deref(doc, obj)? {
        Object::String(b, _) => Some(pdf_string(b)),
        Object::Dictionary(d) => d
            .get(b"UF")
            .ok()
            .and_then(|o| string_of(doc, o))
            .or_else(|| d.get(b"F").ok().and_then(|o| string_of(doc, o)))
            .or_else(|| d.get(b"DOS").ok().and_then(|o| string_of(doc, o)))
            .or_else(|| d.get(b"Unix").ok().and_then(|o| string_of(doc, o))),
        _ => None,
    }
}

const JS_TEXT_CAP: usize = 512 * 1024;

fn js_full(doc: &Document, obj: &Object) -> Option<String> {
    match deref(doc, obj)? {
        Object::String(b, _) => Some(pdf_string_raw(b)),
        Object::Stream(s) => {
            let content = s.decompressed_content_with_limit(256 * 1024).ok()?;
            Some(String::from_utf8_lossy(&content).into_owned())
        }
        _ => None,
    }
}

fn pdf_string_raw(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn note_js(st: &mut Structure, doc: &Document, d: &Dictionary) {
    st.js += 1;
    if st.locked {
        return;
    }
    let Ok(js) = d.get(b"JS") else { return };
    let want_snippet = st.js_snippets.len() < 3;
    let want_text = st.js_text.len() < JS_TEXT_CAP;
    if !want_snippet && !want_text {
        return;
    }
    let Some(full) = js_full(doc, js) else { return };
    if want_snippet {
        let snip = clean_text(&full, SNIPPET);
        if !snip.trim().is_empty() {
            st.js_snippets.push(snip);
        }
    }
    if want_text {
        let room = JS_TEXT_CAP - st.js_text.len();
        let take: String = full.chars().take(room).collect();
        st.js_text.push_str(&take);
        st.js_text.push('\n');
    }
}

const CRITICAL_SCRIPT_SIGNS: &[(&str, &str)] = &[
    ("exportDataObject", "extracts an attachment to disk"),
    ("nLaunch", "launches an extracted attachment"),
    ("customDictionaryOpen", "spell.customDictionaryOpen, a known exploit vector"),
    ("media.newPlayer", "media.newPlayer, a known exploit vector"),
    ("Collab.", "the Collab API, a known exploit vector"),
    ("syncAnnotScan", "syncAnnotScan, a known exploit vector"),
];

const WARNING_SCRIPT_SIGNS: &[(&str, &str)] = &[
    ("util.printf", "util.printf, once a common exploit vector"),
    ("getAnnots", "getAnnots, once a common exploit vector"),
    ("getIcon", "getIcon, once a common exploit vector"),
    ("unescape(", "decodes escaped data"),
    ("fromCharCode", "builds code from character codes"),
    ("eval(", "runs generated code"),
    ("launchURL", "opens a web address"),
    ("submitForm(", "sends form data to a server"),
    ("app.openDoc", "opens another document"),
    ("mailDoc", "sends email"),
    ("saveAs", "writes files"),
];

fn shellcode_escapes(js: &str) -> bool {
    let b = js.as_bytes();
    let mut hits = 0;
    let mut i = 0;
    while i + 6 <= b.len() {
        if b[i] == b'%' && b[i + 1] == b'u' && b[i + 2..i + 6].iter().all(|c| c.is_ascii_hexdigit()) {
            hits += 1;
            if hits >= 4 {
                return true;
            }
            i += 6;
        } else {
            i += 1;
        }
    }
    false
}

fn long_hex_blob(js: &str) -> bool {
    let mut run = 0;
    for c in js.bytes() {
        if c.is_ascii_hexdigit() {
            run += 1;
            if run >= 400 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn script_signs(js: &str) -> (Vec<String>, Vec<String>) {
    let mut critical = Vec::new();
    let mut warning = Vec::new();
    for (needle, why) in CRITICAL_SCRIPT_SIGNS {
        if js.contains(needle) {
            critical.push(format!("{needle}: {why}"));
        }
    }
    if shellcode_escapes(js) {
        critical.push("%u-escaped byte sequences, the shape of shellcode".into());
    }
    if long_hex_blob(js) {
        critical.push("a long blob of hex digits, typical of an embedded payload".into());
    }
    for (needle, why) in WARNING_SCRIPT_SIGNS {
        if js.contains(needle) {
            warning.push(format!("{needle}: {why}"));
        }
    }
    (critical, warning)
}

struct PageMap {
    index_of: HashMap<ObjectId, usize>,
}

fn walk_dicts<'a>(doc: &'a Document, obj: &'a Object, depth: usize, f: &mut dyn FnMut(&'a Dictionary)) {
    if depth > MAX_DEPTH {
        return;
    }
    match obj {
        Object::Dictionary(d) => {
            f(d);
            for (_, v) in d.iter() {
                walk_dicts(doc, v, depth + 1, f);
            }
        }
        Object::Stream(s) => {
            f(&s.dict);
            for (_, v) in s.dict.iter() {
                walk_dicts(doc, v, depth + 1, f);
            }
        }
        Object::Array(items) => {
            for v in items {
                walk_dicts(doc, v, depth + 1, f);
            }
        }
        _ => {}
    }
}

fn walk_name_tree(
    doc: &Document,
    node: &Object,
    depth: usize,
    seen: &mut HashSet<ObjectId>,
    out: &mut Vec<(Vec<u8>, Object)>,
) {
    if depth > MAX_DEPTH || out.len() > 5000 {
        return;
    }
    if let Object::Reference(id) = node {
        if !seen.insert(*id) {
            return;
        }
    }
    let Some(dict) = dict_of(doc, node) else { return };
    if let Ok(Object::Array(names)) = dict.get(b"Names").and_then(|n| doc.dereference(n).map(|(_, o)| o)) {
        for pair in names.chunks_exact(2) {
            if let Ok(key) = pair[0].as_str() {
                out.push((key.to_vec(), pair[1].clone()));
            }
        }
    }
    if let Ok(Object::Array(kids)) = dict.get(b"Kids").and_then(|n| doc.dereference(n).map(|(_, o)| o)) {
        for kid in kids {
            walk_name_tree(doc, kid, depth + 1, seen, out);
        }
    }
}

fn dest_page(doc: &Document, dest: &Object, pages: &PageMap, named: &HashMap<Vec<u8>, Object>, depth: usize) -> Option<usize> {
    if depth > 4 {
        return None;
    }
    match deref(doc, dest)? {
        Object::Array(items) => match items.first()? {
            Object::Reference(id) => pages.index_of.get(id).copied(),
            Object::Integer(i) => usize::try_from(*i).ok(),
            _ => None,
        },
        Object::Dictionary(d) => {
            let inner = d.get(b"D").ok()?;
            dest_page(doc, inner, pages, named, depth + 1)
        }
        Object::String(key, _) | Object::Name(key) => {
            let target = named.get(key)?;
            dest_page(doc, target, pages, named, depth + 1)
        }
        _ => None,
    }
}

fn named_destinations(doc: &Document) -> HashMap<Vec<u8>, Object> {
    let mut map = HashMap::new();
    let Ok(catalog) = doc.catalog() else { return map };
    if let Some(dests) = catalog.get(b"Dests").ok().and_then(|o| dict_of(doc, o)) {
        for (k, v) in dests.iter() {
            map.insert(k.clone(), v.clone());
        }
    }
    if let Some(names) = catalog.get(b"Names").ok().and_then(|o| dict_of(doc, o)) {
        if let Ok(tree) = names.get(b"Dests") {
            let mut out = Vec::new();
            walk_name_tree(doc, tree, 0, &mut HashSet::new(), &mut out);
            for (k, v) in out {
                map.insert(k, v);
            }
        }
    }
    map
}

fn page_map(doc: &Document) -> PageMap {
    let mut index_of = HashMap::new();
    for (num, id) in doc.get_pages() {
        index_of.insert(id, (num as usize).saturating_sub(1));
    }
    PageMap { index_of }
}

fn structural_scan(doc: &Document, locked: bool, encrypted: bool) -> Structure {
    let mut st = Structure { encrypted: encrypted || doc.is_encrypted(), locked, ..Default::default() };
    let pages = page_map(doc);
    let named = named_destinations(doc);
    let mut uris: HashSet<String> = HashSet::new();

    if let Ok(catalog) = doc.catalog() {
        if let Ok(oa) = catalog.get(b"OpenAction") {
            match deref(doc, oa) {
                Some(Object::Dictionary(d)) => {
                    let kind = d.get(b"S").ok().and_then(name_of).unwrap_or_else(|| "action".into());
                    st.open_action = Some(kind);
                }
                Some(Object::Array(_)) => {
                    st.open_dest_page = dest_page(doc, oa, &pages, &named, 0);
                }
                _ => {}
            }
        }
        if let Some(form) = catalog.get(b"AcroForm").ok().and_then(|o| dict_of(doc, o)) {
            st.acroform = true;
            if form.has(b"XFA") {
                st.xfa = true;
            }
        }
        if let Some(names) = catalog.get(b"Names").ok().and_then(|o| dict_of(doc, o)) {
            if let Ok(tree) = names.get(b"EmbeddedFiles") {
                let mut out = Vec::new();
                walk_name_tree(doc, tree, 0, &mut HashSet::new(), &mut out);
                for (key, spec) in out {
                    let name = filespec_name(doc, &spec).unwrap_or_else(|| pdf_string(&key));
                    st.attachments.push(name);
                }
            }
        }
    }

    let mut seen_launch = HashSet::new();
    for (_, obj) in doc.objects.iter() {
        walk_dicts(doc, obj, 0, &mut |d| {
            if let Ok(s) = d.get(b"S") {
                match name_of(s).as_deref() {
                    Some("JavaScript") => note_js(&mut st, doc, d),
                    Some("Launch") => {
                        let target = d
                            .get(b"F")
                            .ok()
                            .and_then(|o| filespec_name(doc, o))
                            .or_else(|| {
                                d.get(b"Win").ok().and_then(|o| dict_of(doc, o)).and_then(|w| w.get(b"F").ok()).and_then(|o| string_of(doc, o))
                            })
                            .unwrap_or_else(|| "(unnamed program)".into());
                        if seen_launch.insert(target.clone()) {
                            st.launch.push(target);
                        }
                    }
                    Some("SubmitForm") => {
                        let url = d.get(b"F").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(unknown destination)".into());
                        if !st.submit.contains(&url) {
                            st.submit.push(url);
                        }
                    }
                    Some("ImportData") => st.import_data += 1,
                    Some("GoToR") | Some("GoToE") => {
                        let f = d.get(b"F").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(embedded document)".into());
                        if !st.gotor.contains(&f) {
                            st.gotor.push(f);
                        }
                    }
                    Some("URI") => {
                        if let Some(u) = d.get(b"URI").ok().and_then(|o| string_of(doc, o)) {
                            if uris.insert(u.clone()) {
                                st.links.push(Link { page: None, kind: "uri".into(), target: u });
                            }
                        }
                    }
                    Some("Rendition") => st.media.push("Rendition action".into()),
                    _ => {}
                }
            } else if d.has(b"JS") {
                note_js(&mut st, doc, d);
            }
            if let Ok(aa) = d.get(b"AA") {
                st.aa += 1;
                if let Some(actions) = dict_of(doc, aa) {
                    for (_, action) in actions.iter() {
                        let is_js = dict_of(doc, action)
                            .and_then(|a| a.get(b"S").ok().and_then(name_of))
                            .map(|k| k == "JavaScript")
                            .unwrap_or(false);
                        if is_js {
                            st.aa_js = true;
                        }
                    }
                }
            }
            if d.has(b"XFA") {
                st.xfa = true;
            }
            if let Some(t) = d.get(b"Type").ok().and_then(name_of) {
                match t.as_str() {
                    "ObjStm" => st.objstm += 1,
                    "Sig" => st.signatures += 1,
                    _ => {}
                }
            }
            if let Some(sub) = d.get(b"Subtype").ok().and_then(name_of) {
                match sub.as_str() {
                    "Widget" => st.widgets += 1,
                    "RichMedia" => st.media.push("Rich media".into()),
                    "3D" => st.media.push("3D model".into()),
                    "Sound" => st.media.push("Sound".into()),
                    "Movie" => st.media.push("Movie".into()),
                    "Screen" => st.media.push("Screen annotation".into()),
                    _ => {}
                }
            }
            if let Ok(filter) = d.get(b"Filter") {
                let has_jbig2 = match deref(doc, filter) {
                    Some(Object::Name(n)) => n.as_slice() == b"JBIG2Decode",
                    Some(Object::Array(a)) => a.iter().any(|f| f.as_name().map(|n| n == b"JBIG2Decode").unwrap_or(false)),
                    _ => false,
                };
                if has_jbig2 {
                    st.jbig2 += 1;
                }
            }
        });
    }

    let mut page_links: Vec<Link> = Vec::new();
    for (num, page_id) in doc.get_pages() {
        let index = (num as usize).saturating_sub(1);
        let Ok(page) = doc.get_dictionary(page_id) else { continue };
        let Some(Object::Array(annots)) = page.get(b"Annots").ok().and_then(|a| deref(doc, a)) else { continue };
        for annot in annots {
            let Some(a) = dict_of(doc, annot) else { continue };
            let sub = a.get(b"Subtype").ok().and_then(name_of).unwrap_or_default();
            match sub.as_str() {
                "Link" => {
                    if let Some(action) = a.get(b"A").ok().and_then(|o| dict_of(doc, o)) {
                        let kind = action.get(b"S").ok().and_then(name_of).unwrap_or_default();
                        match kind.as_str() {
                            "URI" => {
                                if let Some(u) = action.get(b"URI").ok().and_then(|o| string_of(doc, o)) {
                                    page_links.push(Link { page: Some(index), kind: "uri".into(), target: u });
                                }
                            }
                            "GoTo" => {
                                let target = action.get(b"D").ok().and_then(|d| dest_page(doc, d, &pages, &named, 0));
                                page_links.push(Link {
                                    page: Some(index),
                                    kind: "internal".into(),
                                    target: target.map(|p| format!("Page {}", p + 1)).unwrap_or_else(|| "Inside this document".into()),
                                });
                            }
                            "GoToR" | "GoToE" => {
                                let f = action.get(b"F").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(other document)".into());
                                page_links.push(Link { page: Some(index), kind: "file".into(), target: f });
                            }
                            "Launch" => {
                                let f = action.get(b"F").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(program)".into());
                                page_links.push(Link { page: Some(index), kind: "launch".into(), target: f });
                            }
                            "JavaScript" => {
                                page_links.push(Link { page: Some(index), kind: "script".into(), target: "Runs JavaScript".into() });
                            }
                            "SubmitForm" => {
                                let f = action.get(b"F").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(server)".into());
                                page_links.push(Link { page: Some(index), kind: "submit".into(), target: f });
                            }
                            _ => {}
                        }
                    } else if let Ok(dest) = a.get(b"Dest") {
                        let target = dest_page(doc, dest, &pages, &named, 0);
                        page_links.push(Link {
                            page: Some(index),
                            kind: "internal".into(),
                            target: target.map(|p| format!("Page {}", p + 1)).unwrap_or_else(|| "Inside this document".into()),
                        });
                    }
                }
                "FileAttachment" => {
                    let name = a.get(b"FS").ok().and_then(|o| filespec_name(doc, o)).unwrap_or_else(|| "(attached file)".into());
                    if !st.attachments.contains(&name) {
                        st.attachments.push(name);
                    }
                }
                _ => {}
            }
        }
    }

    let mut merged: Vec<Link> = Vec::new();
    let mut seen: HashSet<(Option<usize>, String)> = HashSet::new();
    let mut seen_targets: HashSet<String> = HashSet::new();
    for l in page_links.into_iter().chain(st.links.drain(..)) {
        if l.kind == "uri" && l.page.is_none() && seen_targets.contains(&l.target) {
            continue;
        }
        if seen.insert((l.page, l.target.clone())) {
            seen_targets.insert(l.target.clone());
            merged.push(l);
        }
    }
    st.uri_count = merged.iter().filter(|l| l.kind == "uri").count();
    st.links = merged;
    st
}

fn metadata(doc: &Document) -> Metadata {
    let mut m = Metadata::default();
    let Some(info) = doc.trailer.get(b"Info").ok().and_then(|o| dict_of(doc, o)) else { return m };
    let get = |key: &[u8]| info.get(key).ok().and_then(|o| string_of(doc, o)).filter(|s| !s.is_empty());
    m.title = get(b"Title");
    m.author = get(b"Author");
    m.subject = get(b"Subject");
    m.keywords = get(b"Keywords");
    m.creator = get(b"Creator");
    m.producer = get(b"Producer");
    m.created = get(b"CreationDate").map(|d| pdf_date(&d));
    m.modified = get(b"ModDate").map(|d| pdf_date(&d));
    m
}

fn finding(id: &str, severity: &str, title: impl Into<String>, detail: impl Into<String>, count: usize, evidence: Vec<String>) -> Finding {
    Finding { id: id.into(), severity: severity.into(), title: title.into(), detail: detail.into(), count, evidence }
}

fn severity_rank(s: &str) -> u8 {
    match s {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" => 1,
        _ => 0,
    }
}

pub fn scan(data: &[u8], doc: Option<&Document>, page_count: usize, locked: bool, encrypted: bool) -> ScanReport {
    let started = Instant::now();
    let raw = raw_scan(data);
    let structure = doc.map(|d| structural_scan(d, locked, encrypted));
    let mut f: Vec<Finding> = Vec::new();
    let count = |key: &str| raw.counts.get(key).copied().unwrap_or(0);

    match raw.header_offset {
        None => f.push(finding("no_header", "high", "No PDF header", "The file doesn't start with a PDF signature. It may be a different kind of file wearing a .pdf name.", 1, vec![])),
        Some(off) if off > 0 => f.push(finding("header_offset", "medium", "PDF header is not at the start", format!("The PDF signature appears {off} bytes in. Readers tolerate this, and attackers use the slack to hide another file format in front of the document."), 1, vec![])),
        _ => {}
    }
    if !raw.polyglot.is_empty() {
        f.push(finding("polyglot", "critical", "Looks like more than one file type", "Besides PDF, the bytes also form another valid file format. Polyglot files are a classic way to smuggle executables past filters.", raw.polyglot.len(), raw.polyglot.clone()));
    }

    let raw_js = count("JS").max(count("JavaScript"));
    let js_count = structure.as_ref().map(|s| s.js).unwrap_or(raw_js);
    let open_action = structure.as_ref().map(|s| s.open_action.is_some()).unwrap_or(count("OpenAction") > 0);
    let has_aa = structure.as_ref().map(|s| s.aa > 0).unwrap_or(count("AA") > 0);
    let launch_count = structure.as_ref().map(|s| s.launch.len()).unwrap_or(count("Launch"));

    if js_count > 0 {
        let on_open = structure.as_ref().map(|s| s.open_action.as_deref() == Some("JavaScript")).unwrap_or(open_action);
        let on_event = structure.as_ref().map(|s| s.aa_js).unwrap_or(has_aa);
        let evidence = structure.as_ref().map(|s| s.js_snippets.clone()).unwrap_or_default();
        let (critical_signs, warning_signs) = structure
            .as_ref()
            .map(|s| script_signs(&s.js_text))
            .unwrap_or_default();
        if !critical_signs.is_empty() {
            f.push(finding("script_malicious", "critical", "Script looks malicious", "The script calls functions that only make sense in an attack. Forms use scripts to format and validate fields; they don't do this.", critical_signs.len(), critical_signs));
        } else if !warning_signs.is_empty() {
            f.push(finding("script_suspicious", "high", "Script does more than format fields", "The script reaches outside the document, which is unusual for a form and worth a careful look.", warning_signs.len(), warning_signs));
        }
        if on_open {
            f.push(finding("javascript", "high", "Runs a script when opened", "Code set to run the moment the document opens in a conventional reader. Common in forms that fill in a date or a total, also the first move of most malicious PDFs. Paper Wasp has no script engine, so it never runs here.", js_count, evidence));
        } else if on_event {
            f.push(finding("javascript", "medium", "Runs scripts on form events", "Code bound to fields: typically formatting, validation or calculation when you type. Paper Wasp has no script engine, so it never runs here.", js_count, evidence));
        } else {
            f.push(finding("javascript", "medium", "Contains JavaScript", "Code that a conventional reader could run when you interact with the document. Paper Wasp has no script engine, so it never runs here.", js_count, evidence));
        }
    }
    if js_count == 0 && raw_js > 0 {
        f.push(finding("js_unconfirmed", "low", "Unconfirmed JavaScript keyword", "The byte scan found a JavaScript keyword that the object walk couldn't tie to any script. It is often text on a page or a leftover from an earlier save.", raw_js, vec![]));
    }
    if launch_count > 0 {
        let evidence = structure.as_ref().map(|s| s.launch.clone()).unwrap_or_default();
        f.push(finding("launch", "critical", "Tries to start a program", "A Launch action asks the reader to run an external command or file. Paper Wasp ignores every action in the document.", launch_count, evidence));
    }
    if let Some(s) = &structure {
        if let Some(kind) = &s.open_action {
            if kind != "JavaScript" {
                f.push(finding("open_action", "medium", "Runs an action when opened", format!("A {kind} action is set to fire automatically when the file opens. Paper Wasp only draws pages, so nothing fires."), 1, vec![]));
            }
        } else if let Some(p) = s.open_dest_page {
            f.push(finding("open_dest", "info", "Opens at a specific page", format!("The file asks readers to jump to page {} on open. Harmless.", p + 1), 1, vec![]));
        }
        if s.aa > 0 && !s.aa_js {
            f.push(finding("auto_actions", "medium", "Has event-triggered actions", "Actions bound to events such as a page opening, the cursor entering a field, or the document closing. None of them can trigger here.", s.aa, vec![]));
        }
        if !s.submit.is_empty() {
            f.push(finding("submit_form", "high", "Can send data to a server", "A form submission action posts field contents to a remote address. Paper Wasp never submits anything.", s.submit.len(), s.submit.clone()));
        }
        if s.import_data > 0 {
            f.push(finding("import_data", "medium", "Can import data from disk", "An ImportData action reads form data from a local file in other readers.", s.import_data, vec![]));
        }
        if !s.gotor.is_empty() {
            f.push(finding("remote_goto", "medium", "Points at other documents", "Links that open a different file, possibly fetched from the network. They are listed under Links and never followed automatically.", s.gotor.len(), s.gotor.clone()));
        }
        if !s.attachments.is_empty() {
            let risky: Vec<String> = s
                .attachments
                .iter()
                .filter(|n| {
                    let l = n.to_lowercase();
                    [".exe", ".scr", ".bat", ".cmd", ".com", ".js", ".jse", ".vbs", ".vbe", ".wsf", ".ps1", ".hta", ".jar", ".lnk", ".msi", ".dll", ".docm", ".xlsm", ".pptm", ".sh"]
                        .iter()
                        .any(|ext| l.ends_with(ext))
                })
                .cloned()
                .collect();
            if !risky.is_empty() {
                f.push(finding("attachments", "high", format!("Attached program{}", if risky.len() == 1 { "" } else { "s" }), "An attached file with an executable extension. Paper Wasp never extracts or opens attachments; neither should you.", risky.len(), risky));
            } else {
                f.push(finding("attachments", "medium", format!("Contains {} attached file{}", s.attachments.len(), if s.attachments.len() == 1 { "" } else { "s" }), "Attachments are the most common way to carry an executable inside a PDF. Paper Wasp never extracts or opens them.", s.attachments.len(), s.attachments.clone()));
            }
        }
        if !s.media.is_empty() {
            let mut kinds = s.media.clone();
            kinds.sort();
            kinds.dedup();
            f.push(finding("media", "medium", "Contains embedded media", "Rich media, 3D, sound or video objects need plugins in other readers and have a long exploit history. Here they are flattened to pixels.", s.media.len(), kinds));
        }
        if s.xfa {
            f.push(finding("xfa", "medium", "Uses XFA forms", "An XML forms technology that was repeatedly exploited in the past. Paper Wasp shows only the static appearance.", 1, vec![]));
        }
        if s.acroform && !s.xfa {
            f.push(finding("acroform", "low", "Has fillable form fields", format!("{} interactive field{} rendered flat. Nothing can be typed in or submitted from this reader.", s.widgets, if s.widgets == 1 { "" } else { "s" }), s.widgets.max(1), vec![]));
        }
        if s.uri_count > 0 {
            f.push(finding("links", "low", format!("Contains {} web link{}", s.uri_count, if s.uri_count == 1 { "" } else { "s" }), "Listed under Links. They are inert on the page and only open if you choose to open one yourself.", s.uri_count, vec![]));
        }
        if s.signatures > 0 {
            f.push(finding("signature", "info", "Digitally signed", "The document carries a signature field. Paper Wasp doesn't verify signatures.", s.signatures, vec![]));
        }
        if s.jbig2 > 0 {
            f.push(finding("jbig2", "info", "Uses JBIG2 image compression", "A bilevel image codec that has been used in real exploits against other readers. Decoded here by memory-safe code inside the sandbox.", s.jbig2, vec![]));
        }
        if s.objstm > 0 {
            f.push(finding("objstm", "info", "Uses compressed object streams", "Normal for modern PDFs. Objects inside them are invisible to byte-level scanners, which is why Paper Wasp also walks the parsed structure.", s.objstm, vec![]));
        }
    } else {
        if has_aa {
            f.push(finding("auto_actions", "medium", "Has event-triggered actions", "Actions bound to document or page events. None of them can trigger here.", count("AA"), vec![]));
        }
        if open_action && js_count == 0 {
            f.push(finding("open_action", "medium", "Runs an action when opened", "Something is set to happen automatically when the file opens. Paper Wasp only draws pages.", count("OpenAction"), vec![]));
        }
        for (key, id, sev, title, detail) in [
            ("SubmitForm", "submit_form", "high", "Can send data to a server", "A form submission action posts field contents to a remote address."),
            ("EmbeddedFile", "attachments", "medium", "Contains attached files", "Attachments are the most common way to carry an executable inside a PDF."),
            ("RichMedia", "media", "medium", "Contains embedded media", "Rich media objects need plugins in other readers."),
            ("XFA", "xfa", "medium", "Uses XFA forms", "An XML forms technology that was repeatedly exploited in the past."),
            ("GoToR", "remote_goto", "medium", "Points at other documents", "Links that open a different file."),
            ("AcroForm", "acroform", "low", "Has fillable form fields", "Interactive fields are rendered flat."),
            ("URI", "links", "low", "Contains web links", "Links are inert on the page."),
        ] {
            let n = count(key);
            if n > 0 {
                f.push(finding(id, sev, title, detail, n, vec![]));
            }
        }
    }

    let encrypted = structure.as_ref().map(|s| s.encrypted).unwrap_or(false) || count("Encrypt") > 0 || locked;
    if encrypted && locked {
        let detail = if structure.is_some() {
            "The contents are encrypted until the file is unlocked, so scripts and text inside can't be read yet. The structure was still inspected."
        } else {
            "The contents are encrypted until the file is unlocked, so they couldn't be inspected yet."
        };
        f.push(finding("locked", "low", "Password-protected", detail, 1, vec![]));
    } else if encrypted && structure.is_some() {
        f.push(finding("encrypted", "info", "Encrypted", "The file is encrypted and was unlocked for reading. Everything inside was inspected.", 1, vec![]));
    } else if encrypted {
        f.push(finding("encrypted", "info", "Encrypted", "The file is encrypted and its structure couldn't be parsed, so the contents couldn't be inspected.", 1, vec![]));
    }
    if !raw.obfuscated.is_empty() {
        let evidence: Vec<String> = raw.obfuscated.iter().map(|(k, n)| format!("/{k} written with hex escapes ×{n}")).collect();
        f.push(finding("obfuscation", "critical", "Keywords are hex-escaped", "Names such as /J#61vaScript are spelled with escape codes so simple scanners miss them. There is no honest reason to do this.", raw.obfuscated.values().sum(), evidence));
    }
    if raw.trailing > 512 {
        f.push(finding("trailing_data", "low", "Extra data after the end of file", format!("{} bytes follow the final end-of-file marker. Often harmless padding, sometimes a hidden payload.", raw.trailing), 1, vec![]));
    }
    if raw.eof_count > 1 {
        f.push(finding("incremental", "info", format!("Saved {} times", raw.eof_count), "The file contains incremental updates, so earlier versions of the content may still be inside it.", raw.eof_count, vec![]));
    }
    if structure.is_none() {
        f.push(finding("unparsed", "low", "Structure couldn't be parsed cleanly", "The cross-reference table didn't parse, so these findings come from a byte-level scan only. Malformed structure is common in both sloppy and malicious files.", 1, vec![]));
    }
    if page_count == 0 && doc.is_some() && !locked {
        f.push(finding("no_pages", "medium", "No renderable pages", "The page tree couldn't be read. A PDF with nothing to show is unusual.", 1, vec![]));
    }

    f.sort_by(|a, b| severity_rank(&b.severity).cmp(&severity_rank(&a.severity)));
    let top = f.iter().map(|x| severity_rank(&x.severity)).max().unwrap_or(0);
    let verdict = match top {
        5 => "hostile",
        4 => "warning",
        3 => "caution",
        _ => "clean",
    };

    let mut counts = raw.counts.clone();
    counts.insert("pages".into(), page_count);
    let version = doc.map(|d| d.version.clone()).filter(|v| !v.is_empty()).unwrap_or(raw.version.clone());

    ScanReport {
        verdict: verdict.into(),
        findings: f,
        structure_parsed: structure.is_some(),
        encrypted,
        version,
        metadata: doc.map(metadata).unwrap_or_default(),
        links: structure.as_ref().map(|s| s.links.clone()).unwrap_or_default(),
        attachments: structure.as_ref().map(|s| s.attachments.clone()).unwrap_or_default(),
        counts,
        object_count: doc.map(|d| d.objects.len()).unwrap_or(raw.objects),
        stream_count: raw.streams,
        incremental_updates: raw.eof_count.saturating_sub(1),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

pub fn outline(doc: &Document) -> Vec<OutlineItem> {
    let Ok(catalog) = doc.catalog() else { return vec![] };
    let Some(root) = catalog.get(b"Outlines").ok().and_then(|o| dict_of(doc, o)) else { return vec![] };
    let Ok(first) = root.get(b"First") else { return vec![] };
    let pages = page_map(doc);
    let named = named_destinations(doc);
    let mut seen = HashSet::new();
    let mut budget = 4000usize;
    walk_outline(doc, first, 0, &mut seen, &pages, &named, &mut budget)
}

fn walk_outline(
    doc: &Document,
    start: &Object,
    depth: usize,
    seen: &mut HashSet<ObjectId>,
    pages: &PageMap,
    named: &HashMap<Vec<u8>, Object>,
    budget: &mut usize,
) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    if depth > 12 {
        return items;
    }
    let mut node = start.clone();
    loop {
        if *budget == 0 {
            break;
        }
        if let Object::Reference(id) = node {
            if !seen.insert(id) {
                break;
            }
        }
        let Some(dict) = dict_of(doc, &node) else { break };
        *budget -= 1;
        let title = dict.get(b"Title").ok().and_then(|t| string_of(doc, t)).unwrap_or_default();
        let page = dict
            .get(b"Dest")
            .ok()
            .and_then(|d| dest_page(doc, d, pages, named, 0))
            .or_else(|| {
                let a = dict.get(b"A").ok().and_then(|o| dict_of(doc, o))?;
                let kind = a.get(b"S").ok().and_then(name_of)?;
                if kind == "GoTo" {
                    dest_page(doc, a.get(b"D").ok()?, pages, named, 0)
                } else {
                    None
                }
            });
        let children = match dict.get(b"First") {
            Ok(first) => walk_outline(doc, first, depth + 1, seen, pages, named, budget),
            Err(_) => Vec::new(),
        };
        items.push(OutlineItem { title, page, children });
        match dict.get(b"Next") {
            Ok(next) => node = next.clone(),
            Err(_) => break,
        }
    }
    items
}
