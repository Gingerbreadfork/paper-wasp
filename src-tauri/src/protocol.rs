//! Wire types shared by the host and the sandboxed worker.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Status,
    Load {
        len: usize,
        #[serde(default)]
        password: Option<String>,
    },
    Unload,
    Render { page: usize, scale: f32 },
    Text { page: usize },
    Scan,
    Outline,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReplyHeader {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub len: usize,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LoadInfo {
    pub pages: Vec<PageSize>,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_error: Option<String>,
    #[serde(default)]
    pub needs_password: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SandboxStatus {
    pub landlock: String,
    pub seccomp: bool,
    pub network_isolated: bool,
    pub no_new_privs: bool,
    pub memory_cap_mb: u64,
    pub fs_locked: bool,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub severity: String,
    pub count: usize,
    pub evidence: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub page: Option<usize>,
    pub kind: String,
    pub target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ScanReport {
    pub verdict: String,
    pub findings: Vec<Finding>,
    pub structure_parsed: bool,
    pub encrypted: bool,
    pub version: String,
    pub metadata: Metadata,
    pub links: Vec<Link>,
    pub attachments: Vec<String>,
    pub counts: BTreeMap<String, usize>,
    pub object_count: usize,
    pub stream_count: usize,
    pub incremental_updates: usize,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OutlineItem {
    pub title: String,
    pub page: Option<usize>,
    pub children: Vec<OutlineItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TextLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub xs: Vec<f32>,
}
