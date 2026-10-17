//! Spawns and supervises sandboxed worker processes.

use crate::protocol::{LoadInfo, ReplyHeader, Request, SandboxStatus, ScanReport};
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

#[derive(Debug)]
pub enum WorkerError {
    Spawn(String),
    Dead(String),
    Timeout,
    Remote(String),
    NoDocument,
    Protocol(String),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerError::Spawn(e) => write!(f, "The sandboxed renderer couldn't start: {e}"),
            WorkerError::Dead(e) => write!(f, "{e}"),
            WorkerError::Timeout => write!(f, "The renderer took too long and was stopped."),
            WorkerError::Remote(e) => write!(f, "{e}"),
            WorkerError::NoDocument => write!(f, "No document is open."),
            WorkerError::Protocol(e) => write!(f, "Unexpected reply from the renderer: {e}"),
        }
    }
}

impl WorkerError {
    fn is_fatal(&self) -> bool {
        matches!(self, WorkerError::Dead(_) | WorkerError::Timeout | WorkerError::Spawn(_))
    }
}

pub struct Reply {
    pub header: ReplyHeader,
    pub payload: Vec<u8>,
}

enum Frame {
    Reply(Reply),
    Eof,
}

#[cfg(windows)]
mod job {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicUIRestrictions,
        JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_BASIC_UI_RESTRICTIONS,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOB_OBJECT_UILIMIT_DESKTOP, JOB_OBJECT_UILIMIT_DISPLAYSETTINGS,
        JOB_OBJECT_UILIMIT_EXITWINDOWS, JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES,
        JOB_OBJECT_UILIMIT_READCLIPBOARD, JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS,
        JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    };

    pub const MEMORY_CAP_BYTES: usize = 2 * 1024 * 1024 * 1024;

    pub struct Job(HANDLE);

    unsafe impl Send for Job {}

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub fn confine(child: &Child) -> Result<Job, String> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err("CreateJobObject failed".into());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            limits.BasicLimitInformation.ActiveProcessLimit = 1;
            limits.ProcessMemoryLimit = MEMORY_CAP_BYTES;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job);
                return Err("SetInformationJobObject failed".into());
            }
            let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
                UIRestrictionsClass: JOB_OBJECT_UILIMIT_DESKTOP
                    | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
                    | JOB_OBJECT_UILIMIT_EXITWINDOWS
                    | JOB_OBJECT_UILIMIT_GLOBALATOMS
                    | JOB_OBJECT_UILIMIT_HANDLES
                    | JOB_OBJECT_UILIMIT_READCLIPBOARD
                    | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
                    | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
            };
            SetInformationJobObject(
                job,
                JobObjectBasicUIRestrictions,
                &ui as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
            );
            if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0 {
                CloseHandle(job);
                return Err("AssignProcessToJobObject failed".into());
            }
            Ok(Job(job))
        }
    }
}

pub struct WorkerProc {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Frame>,
    pub loaded: Option<(u64, LoadInfo)>,
    pub status: Option<SandboxStatus>,
    #[cfg(windows)]
    job: Option<job::Job>,
}

impl WorkerProc {
    pub fn spawn() -> Result<Self, WorkerError> {
        let exe = std::env::current_exe().map_err(|e| WorkerError::Spawn(e.to_string()))?;
        let mut command = Command::new(exe);
        command.arg("--wasp-worker").env_clear();
        #[cfg(windows)]
        for key in ["SystemRoot", "SystemDrive", "windir", "TEMP", "TMP"] {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| WorkerError::Spawn(e.to_string()))?;
        #[cfg(windows)]
        let job = match job::confine(&child) {
            Ok(j) => Some(j),
            Err(e) => {
                log::warn!("worker job object not applied: {e}");
                None
            }
        };
        let stdin = child.stdin.take().ok_or_else(|| WorkerError::Spawn("no stdin".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| WorkerError::Spawn("no stdout".into()))?;
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("wasp-worker-reader".into())
            .spawn(move || {
                let mut reader = BufReader::with_capacity(1 << 20, stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let header: ReplyHeader = match serde_json::from_str(line.trim_end()) {
                        Ok(h) => h,
                        Err(_) => break,
                    };
                    let mut payload = vec![0u8; header.len];
                    if header.len > 0 && reader.read_exact(&mut payload).is_err() {
                        break;
                    }
                    if tx.send(Frame::Reply(Reply { header, payload })).is_err() {
                        return;
                    }
                }
                let _ = tx.send(Frame::Eof);
            })
            .map_err(|e| WorkerError::Spawn(e.to_string()))?;
        Ok(Self {
            child,
            stdin,
            rx,
            loaded: None,
            status: None,
            #[cfg(windows)]
            job,
        })
    }

    pub fn request(&mut self, req: &Request, payload: &[u8], timeout: Duration) -> Result<Reply, WorkerError> {
        let mut buf = serde_json::to_vec(req).map_err(|e| WorkerError::Protocol(e.to_string()))?;
        buf.push(b'\n');
        let written = self
            .stdin
            .write_all(&buf)
            .and_then(|_| self.stdin.write_all(payload))
            .and_then(|_| self.stdin.flush());
        if let Err(e) = written {
            self.kill();
            return Err(WorkerError::Dead(format!("The renderer stopped unexpectedly ({e}).")));
        }
        match self.rx.recv_timeout(timeout) {
            Ok(Frame::Reply(r)) => {
                if r.header.ok {
                    Ok(r)
                } else {
                    Err(WorkerError::Remote(r.header.error.clone().unwrap_or_else(|| "unknown renderer error".into())))
                }
            }
            Ok(Frame::Eof) | Err(RecvTimeoutError::Disconnected) => {
                self.kill();
                Err(WorkerError::Dead("The renderer stopped unexpectedly. The sandbox contained it and a fresh one will take over.".into()))
            }
            Err(RecvTimeoutError::Timeout) => {
                self.kill();
                Err(WorkerError::Timeout)
            }
        }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for WorkerProc {
    fn drop(&mut self) {
        self.kill();
    }
}

pub struct Doc {
    pub id: u64,
    pub bytes: Arc<Vec<u8>>,
    pub password: Option<String>,
}

pub struct Pool {
    slots: Vec<Mutex<Option<WorkerProc>>>,
    batch: Mutex<Option<WorkerProc>>,
    current: RwLock<Option<Arc<Doc>>>,
}

impl Pool {
    pub fn new(size: usize) -> Self {
        Self {
            slots: (0..size.max(1)).map(|_| Mutex::new(None)).collect(),
            batch: Mutex::new(None),
            current: RwLock::new(None),
        }
    }

    pub fn set_document(&self, doc: Option<Arc<Doc>>) {
        *self.current.write().unwrap_or_else(|p| p.into_inner()) = doc;
    }

    pub fn current(&self) -> Option<Arc<Doc>> {
        self.current.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn release_workers(&self) {
        for slot in &self.slots {
            if let Ok(mut g) = slot.try_lock() {
                if let Some(w) = g.as_mut() {
                    let _ = w.request(&Request::Unload, &[], Duration::from_secs(5));
                    w.loaded = None;
                }
            }
        }
    }

    fn acquire(&self, prefer: usize) -> MutexGuard<'_, Option<WorkerProc>> {
        let n = self.slots.len();
        for i in 0..n {
            let idx = (prefer + i) % n;
            if let Ok(g) = self.slots[idx].try_lock() {
                return g;
            }
        }
        self.slots[prefer % n].lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn call<T>(
        &self,
        prefer: usize,
        need_doc: bool,
        f: impl Fn(&mut WorkerProc) -> Result<T, WorkerError>,
    ) -> Result<T, WorkerError> {
        let mut guard = self.acquire(prefer);
        let mut last = None;
        for attempt in 0..2 {
            match self.call_once(&mut guard, need_doc, &f) {
                Err(WorkerError::Dead(msg)) if attempt == 0 => {
                    log::warn!("sandbox worker died ({msg}); starting a fresh one");
                    last = Some(WorkerError::Dead(msg));
                }
                other => return other,
            }
        }
        Err(last.unwrap_or_else(|| WorkerError::Dead("The renderer could not be restarted.".into())))
    }

    fn call_once<T>(
        &self,
        guard: &mut MutexGuard<'_, Option<WorkerProc>>,
        need_doc: bool,
        f: &impl Fn(&mut WorkerProc) -> Result<T, WorkerError>,
    ) -> Result<T, WorkerError> {
        if guard.is_none() {
            **guard = Some(WorkerProc::spawn()?);
        }
        if need_doc {
            let doc = self.current().ok_or(WorkerError::NoDocument)?;
            let worker = guard.as_mut().expect("worker present");
            if worker.loaded.as_ref().map(|(id, _)| *id) != Some(doc.id) {
                let load = worker.request(&Request::Load { len: doc.bytes.len(), password: doc.password.clone() }, &doc.bytes, Duration::from_secs(180));
                match load {
                    Ok(reply) => {
                        let info: LoadInfo = serde_json::from_value(reply.header.data)
                            .map_err(|e| WorkerError::Protocol(e.to_string()))?;
                        worker.loaded = Some((doc.id, info));
                    }
                    Err(e) => {
                        if e.is_fatal() {
                            **guard = None;
                        }
                        return Err(e);
                    }
                }
            }
        }
        let worker = guard.as_mut().expect("worker present");
        match f(worker) {
            Err(e) if e.is_fatal() => {
                **guard = None;
                Err(e)
            }
            other => other,
        }
    }

    pub fn check(&self, bytes: &[u8]) -> Result<(LoadInfo, ScanReport), WorkerError> {
        let mut guard = self.batch.lock().unwrap_or_else(|p| p.into_inner());
        let mut last = None;
        for attempt in 0..2 {
            if guard.is_none() {
                *guard = Some(WorkerProc::spawn()?);
            }
            let worker = guard.as_mut().expect("worker present");
            let result: Result<(LoadInfo, ScanReport), WorkerError> = (|| {
                let load = worker.request(&Request::Load { len: bytes.len(), password: None }, bytes, Duration::from_secs(120))?;
                let info: LoadInfo =
                    serde_json::from_value(load.header.data).map_err(|e| WorkerError::Protocol(e.to_string()))?;
                let scan = worker.request(&Request::Scan, &[], Duration::from_secs(120))?;
                let report: ScanReport =
                    serde_json::from_value(scan.header.data).map_err(|e| WorkerError::Protocol(e.to_string()))?;
                let _ = worker.request(&Request::Unload, &[], Duration::from_secs(10));
                worker.loaded = None;
                Ok((info, report))
            })();
            match result {
                Err(e) if e.is_fatal() => {
                    *guard = None;
                    if attempt == 0 && matches!(e, WorkerError::Dead(_)) {
                        log::warn!("batch worker died ({e}); starting a fresh one");
                        last = Some(e);
                        continue;
                    }
                    return Err(e);
                }
                other => return other,
            }
        }
        Err(last.unwrap_or_else(|| WorkerError::Dead("The checker could not be restarted.".into())))
    }

    pub fn release_batch_worker(&self) {
        if let Ok(mut g) = self.batch.try_lock() {
            *g = None;
        }
    }

    pub fn sandbox_status(&self) -> Result<SandboxStatus, WorkerError> {
        self.call(0, false, |w| {
            if let Some(s) = &w.status {
                return Ok(s.clone());
            }
            let reply = w.request(&Request::Status, &[], Duration::from_secs(15))?;
            #[allow(unused_mut)]
            let mut status: SandboxStatus =
                serde_json::from_value(reply.header.data).map_err(|e| WorkerError::Protocol(e.to_string()))?;
            #[cfg(windows)]
            {
                let confined = w.job.is_some();
                status.no_child_processes = confined;
                status.memory_cap_mb = if confined { (job::MEMORY_CAP_BYTES / (1024 * 1024)) as u64 } else { 0 };
                if !confined {
                    status.notes.push("The worker could not be placed in a job object; memory and process limits are off.".into());
                }
            }
            w.status = Some(status.clone());
            Ok(status)
        })
    }
}
