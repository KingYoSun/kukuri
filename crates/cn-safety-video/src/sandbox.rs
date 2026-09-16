//! Linux decoder confinement; non-Linux callers cannot silently weaken it.
use std::io;

#[cfg(target_os = "linux")]
pub fn configure(command: &mut tokio::process::Command, max_file_bytes: usize) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
    let parent_pid = std::process::id();
    unsafe {
        command.as_std_mut().pre_exec(move || {
            for (resource, limit) in [
                (libc::RLIMIT_AS, 512 * 1024 * 1024),
                (libc::RLIMIT_CPU, 30),
                (libc::RLIMIT_CORE, 0),
                (libc::RLIMIT_FSIZE, max_file_bytes as u64),
            ] {
                let value = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(resource, &value) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() as u32 != parent_pid {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "decoder parent exited",
                ));
            }
            install_network_filter()
        });
    }
}

#[cfg(target_os = "linux")]
unsafe fn install_network_filter() -> io::Result<()> {
    // seccomp_data: nr at 0, audit architecture at 4. Refuse a different ABI.
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "unsupported decoder sandbox architecture",
    ));
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        let statement = |code, k| libc::sock_filter {
            code,
            jt: 0,
            jf: 0,
            k,
        };
        let jump = |k, jt, jf| libc::sock_filter {
            code: 0x15,
            jt,
            jf,
            k,
        };
        let mut filters = [
            statement(0x20, 4),
            jump(ARCH, 1, 0),
            statement(0x06, 0x80000000),
            statement(0x20, 0),
            statement(0x54, !0x40000000),
            jump(libc::SYS_socket as u32, 3, 0),
            jump(libc::SYS_connect as u32, 2, 0),
            jump(libc::SYS_sendto as u32, 1, 0),
            statement(0x06, 0x7fff0000),
            statement(0x06, 0x00050000 | libc::EPERM as u32),
        ];
        let program = libc::sock_fprog {
            len: filters.len() as u16,
            filter: filters.as_mut_ptr(),
        };
        if unsafe { libc::prctl(libc::PR_SET_SECCOMP, 2, &program) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn configure(_command: &mut tokio::process::Command, _max_file_bytes: usize) {}

#[cfg(target_os = "linux")]
pub fn require_tmpfs(path: &std::path::Path) -> io::Result<()> {
    use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
    let meta = std::fs::metadata(path)?;
    if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "video directory must be private and owned",
        ));
    }
    let name = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid tmpfs path"))?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(name.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { stat.assume_init() }.f_type != 0x01021994 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "video input requires tmpfs",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn require_tmpfs(_path: &std::path::Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "video extraction requires the Linux sandbox",
    ))
}

pub fn kill_group(id: Option<u32>) {
    #[cfg(target_os = "linux")]
    if let Some(id) = id {
        unsafe {
            libc::kill(-(id as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = id;
}
