//! Locks the worker process down before it touches untrusted bytes.

use crate::protocol::SandboxStatus;

#[cfg(target_os = "linux")]
pub fn lock_down() -> SandboxStatus {
    let mut st = SandboxStatus { platform: "linux".into(), ..Default::default() };

    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0);
    }

    st.network_isolated = unshare_namespaces(&mut st.notes);
    st.memory_cap_mb = apply_rlimits(&mut st.notes);
    st.no_new_privs = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0 };
    apply_landlock(&mut st);
    st.seccomp = apply_seccomp(&mut st.notes);
    st.no_child_processes = st.seccomp;
    st
}

#[cfg(not(target_os = "linux"))]
pub fn lock_down() -> SandboxStatus {
    let platform = if cfg!(windows) { "windows" } else { std::env::consts::OS };
    SandboxStatus {
        platform: platform.into(),
        landlock: "unavailable".into(),
        notes: if cfg!(windows) {
            vec!["File and network isolation are not available on Windows yet.".into()]
        } else {
            vec!["Process sandboxing is only implemented on Linux.".into()]
        },
        ..Default::default()
    }
}

#[cfg(target_os = "linux")]
fn unshare_namespaces(notes: &mut Vec<String>) -> bool {
    let flags = libc::CLONE_NEWUSER | libc::CLONE_NEWNET | libc::CLONE_NEWIPC | libc::CLONE_NEWUTS;
    if unsafe { libc::unshare(flags) } == 0 {
        return true;
    }
    let err = std::io::Error::last_os_error();
    notes.push(format!("Network namespace unavailable: {err}"));
    false
}

#[cfg(target_os = "linux")]
fn apply_rlimits(notes: &mut Vec<String>) -> u64 {
    const GIB: u64 = 1024 * 1024 * 1024;
    let data_cap = 2 * GIB;
    let mut ok = true;
    ok &= set_limit(libc::RLIMIT_DATA, data_cap);
    ok &= set_limit(libc::RLIMIT_AS, 6 * GIB);
    ok &= set_limit(libc::RLIMIT_FSIZE, 0);
    ok &= set_limit(libc::RLIMIT_CORE, 0);
    ok &= set_limit(libc::RLIMIT_NOFILE, 16);
    if !ok {
        notes.push("Some resource limits could not be applied.".into());
    }
    data_cap / (1024 * 1024)
}

#[cfg(target_os = "linux")]
fn set_limit(resource: libc::__rlimit_resource_t, value: u64) -> bool {
    let lim = libc::rlimit { rlim_cur: value as libc::rlim_t, rlim_max: value as libc::rlim_t };
    unsafe { libc::setrlimit(resource, &lim) == 0 }
}

#[cfg(target_os = "linux")]
fn apply_landlock(st: &mut SandboxStatus) {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, Ruleset, RulesetAttr,
        RulesetStatus, Scope,
    };

    let abi = ABI::V6;
    let result = (|| -> Result<landlock::RestrictionStatus, landlock::RulesetError> {
        Ruleset::default()
            .set_compatibility(CompatLevel::BestEffort)
            .handle_access(AccessFs::from_all(abi))?
            .handle_access(AccessNet::BindTcp | AccessNet::ConnectTcp)?
            .scope(Scope::AbstractUnixSocket | Scope::Signal)?
            .create()?
            .restrict_self()
    })();

    match result {
        Ok(status) => {
            st.landlock = match status.ruleset {
                RulesetStatus::FullyEnforced => "enforced".into(),
                RulesetStatus::PartiallyEnforced => "partial".into(),
                RulesetStatus::NotEnforced => "unavailable".into(),
            };
            st.fs_locked = status.ruleset != RulesetStatus::NotEnforced;
            if !st.fs_locked {
                st.notes.push("Landlock is not available on this kernel.".into());
            }
        }
        Err(e) => {
            st.landlock = "unavailable".into();
            st.notes.push(format!("Landlock failed: {e}"));
        }
    }
}

#[cfg(target_os = "linux")]
fn apply_seccomp(notes: &mut Vec<String>) -> bool {
    use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, SeccompRule, TargetArch};
    use std::collections::BTreeMap;

    #[cfg(target_arch = "x86_64")]
    let arch = TargetArch::x86_64;
    #[cfg(target_arch = "aarch64")]
    let arch = TargetArch::aarch64;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        notes.push("seccomp filter not built for this architecture.".into());
        return false;
    }

    let mut denied: Vec<i64> = vec![
        libc::SYS_execve,
        libc::SYS_execveat,
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_connect,
        libc::SYS_bind,
        libc::SYS_listen,
        libc::SYS_accept,
        libc::SYS_accept4,
        libc::SYS_sendto,
        libc::SYS_sendmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvfrom,
        libc::SYS_recvmsg,
        libc::SYS_recvmmsg,
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_setns,
        libc::SYS_unshare,
        libc::SYS_kexec_load,
        libc::SYS_init_module,
        libc::SYS_finit_module,
        libc::SYS_delete_module,
        libc::SYS_reboot,
        libc::SYS_open_by_handle_at,
        libc::SYS_io_uring_setup,
        libc::SYS_bpf,
        libc::SYS_userfaultfd,
        libc::SYS_perf_event_open,
        libc::SYS_keyctl,
        libc::SYS_add_key,
        libc::SYS_request_key,
        libc::SYS_openat,
        libc::SYS_openat2,
        libc::SYS_mknodat,
        libc::SYS_setuid,
        libc::SYS_setgid,
        libc::SYS_setresuid,
        libc::SYS_setresgid,
    ];
    #[cfg(target_arch = "x86_64")]
    denied.extend_from_slice(&[
        libc::SYS_open,
        libc::SYS_creat,
        libc::SYS_fork,
        libc::SYS_vfork,
        libc::SYS_mknod,
    ]);

    let rules: BTreeMap<i64, Vec<SeccompRule>> = denied.into_iter().map(|nr| (nr, Vec::new())).collect();
    let filter = match SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    ) {
        Ok(f) => f,
        Err(e) => {
            notes.push(format!("seccomp filter rejected: {e}"));
            return false;
        }
    };
    let program: BpfProgram = match filter.try_into() {
        Ok(p) => p,
        Err(e) => {
            notes.push(format!("seccomp compile failed: {e}"));
            return false;
        }
    };
    match seccompiler::apply_filter(&program) {
        Ok(()) => true,
        Err(e) => {
            notes.push(format!("seccomp apply failed: {e}"));
            false
        }
    }
}
