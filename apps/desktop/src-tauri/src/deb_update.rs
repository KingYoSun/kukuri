//! One polkit attempt; no password handling, shell, sudo or GUI authentication fallback.
use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
use std::{
    fs::File,
    io::Write,
    os::fd::AsRawFd,
    process::{Command, Stdio},
};

struct SealedDeb(File);

impl SealedDeb {
    fn new(bytes: &[u8]) -> Result<Self, String> {
        if !bytes.starts_with(b"!<arch>\n") {
            return Err("deb_update_format_mismatch".into());
        }
        let fd = memfd_create(
            c"kukuri-update.deb",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .map_err(|_| "deb_update_seal_failed")?;
        let mut file = File::from(fd);
        file.write_all(bytes)
            .map_err(|_| "deb_update_seal_failed")?;
        fcntl_add_seals(
            &file,
            SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL,
        )
        .map_err(|_| "deb_update_seal_failed")?;
        Ok(Self(file))
    }

    fn path(&self) -> String {
        // Root dpkg opens the parent's sealed descriptor. No mutable temporary pathname
        // crosses the privilege boundary, and CLOEXEC does not affect this parent handle.
        format!("/proc/{}/fd/{}", std::process::id(), self.0.as_raw_fd())
    }
}

fn check_metadata(output: &[u8], version: &str) -> Result<(), String> {
    if output != format!("kukuri\n{version}\namd64\n").as_bytes() {
        return Err("deb_update_package_mismatch".into());
    }
    Ok(())
}

fn apply_once(mut invoke: impl FnMut() -> std::io::Result<Option<i32>>) -> Result<(), String> {
    match invoke() {
        Ok(Some(0)) => Ok(()),
        Ok(Some(126)) => Err("deb_update_auth_cancelled".into()),
        Ok(Some(127)) | Err(_) => Err("deb_update_auth_unavailable".into()),
        _ => Err("deb_update_install_failed".into()),
    }
}

pub(crate) fn install(bytes: &[u8], version: &str) -> Result<(), String> {
    if rustix::process::geteuid().is_root() {
        return Err("deb_update_root_forbidden".into());
    }
    let sealed = SealedDeb::new(bytes)?;
    let path = sealed.path();
    let metadata = Command::new("/usr/bin/dpkg-deb")
        .args([
            "--show",
            "--showformat=${Package}\n${Version}\n${Architecture}\n",
            &path,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "deb_update_package_mismatch")?;
    if !metadata.status.success() {
        return Err("deb_update_package_mismatch".into());
    }
    check_metadata(&metadata.stdout, version)?;
    apply_once(|| {
        Command::new("/usr/bin/pkexec")
            .args([
                "--disable-internal-agent",
                "/usr/bin/dpkg",
                "--install",
                &path,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.code())
    })?;
    // dpkg success alone is not enough to claim the expected package is configured.
    let actual = Command::new("/usr/bin/dpkg-query")
        .args([
            "--show",
            "--showformat=${db:Status-Status}\n${Version}\n",
            "kukuri",
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "deb_update_install_failed")?;
    if !actual.status.success() || actual.stdout != format!("installed\n{version}\n").as_bytes() {
        return Err("deb_update_install_failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};

    #[test]
    fn verified_bytes_cannot_change_before_or_during_privileged_open() {
        let bytes = b"!<arch>\nverified-deb-fixture";
        let mut sealed = SealedDeb::new(bytes).unwrap();
        let mut second = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(sealed.path())
            .unwrap();
        assert!(second.write_all(b"tampered").is_err());
        assert!(second.set_len(0).is_err());
        assert!(second.set_len(1000).is_err());
        assert!(sealed.0.write_all(b"changed").is_err());
        second.seek(SeekFrom::Start(0)).unwrap();
        let mut actual = Vec::new();
        second.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, bytes);
        // The system reader opens exactly the descriptor later passed to root dpkg.
        let output = Command::new("/usr/bin/cat")
            .arg(sealed.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, bytes);
    }

    #[test]
    fn cancellation_refusal_missing_agent_and_partial_failure_never_retry() {
        for code in [Some(126), Some(127), Some(1), None] {
            let mut authentication_attempts = 0;
            let mut restarts = 0;
            let result = apply_once(|| {
                authentication_attempts += 1;
                Ok(code)
            });
            if result.is_ok() {
                restarts += 1;
            }
            assert!(result.is_err());
            assert_eq!(authentication_attempts, 1);
            assert_eq!(restarts, 0);
        }
        let mut attempts = 0;
        assert!(
            apply_once(|| {
                attempts += 1;
                Err(std::io::ErrorKind::NotFound.into())
            })
            .is_err()
        );
        assert_eq!(attempts, 1);
        assert!(apply_once(|| Ok(Some(0))).is_ok());
    }

    #[test]
    fn different_format_package_version_or_arch_never_reaches_authentication() {
        assert!(SealedDeb::new(b"\x7fELFAppImage").is_err());
        assert!(check_metadata(b"kukuri\n0.1.9\namd64\n", "0.1.9").is_ok());
        for output in [
            b"kukuri\n0.1.8\namd64\n".as_slice(),
            b"kukuri\n0.1.9\narm64\n",
            b"another\n0.1.9\namd64\n",
        ] {
            assert!(check_metadata(output, "0.1.9").is_err());
        }
    }
}
