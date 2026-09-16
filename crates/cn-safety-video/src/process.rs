use kukuri_cn_safety::ScanError;
use std::{ffi::OsString, path::Path, process::Stdio};
use tokio::{io::AsyncReadExt, process::Command, sync::watch, time::Instant};

use crate::config::invalid;

async fn bounded_output(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>, ScanError> {
    let mut bytes = Vec::new();
    reader
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| invalid("decoder output read failed"))?;
    if bytes.len() > 64 * 1024 {
        return Err(invalid("decoder diagnostic output exceeds limit"));
    }
    Ok(bytes)
}

pub async fn run(
    program: &Path,
    args: &[OsString],
    directory: &Path,
    max_file_bytes: usize,
    deadline: Instant,
    cancel: &mut watch::Receiver<bool>,
) -> Result<Vec<u8>, ScanError> {
    if *cancel.borrow() {
        return Err(ScanError::Unavailable("video scan cancelled".into()));
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(directory)
        .env_clear()
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::sandbox::configure(&mut command, max_file_bytes);
    let mut child = command
        .spawn()
        .map_err(|_| ScanError::Unavailable("cannot start sandboxed decoder".into()))?;
    let id = child.id();
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let result = tokio::select! {
        result = async {
            let (stdout,_,status) = tokio::try_join!(bounded_output(stdout), bounded_output(stderr),
                async {child.wait().await.map_err(|_| invalid("decoder wait failed"))})?;
            if !status.success() { return Err(invalid("decoder failed or exceeded resource limits")); }
            Ok(stdout)
        } => result,
        _ = tokio::time::sleep_until(deadline) => Err(ScanError::Timeout("video decoder deadline exceeded".into())),
        _ = cancel.changed() => Err(ScanError::Unavailable("video scan cancelled".into())),
    };
    // Also terminate descendants holding pipes after their original child exited.
    crate::sandbox::kill_group(id);
    let _ = child.kill().await;
    let _ = child.wait().await;
    result
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn decoder_process_cannot_open_network_sockets() {
        let root = tempfile::tempdir_in("/dev/shm").expect("tmpfs");
        let (_tx, mut rx) = watch::channel(false);
        let output=run(Path::new("/usr/bin/python3"),&[
            "-c".into(),"import socket\ntry:\n socket.socket()\n print('allowed')\nexcept PermissionError:\n print('blocked')".into()
        ],root.path(),1024,Instant::now()+std::time::Duration::from_secs(5),&mut rx).await.expect("sandboxed Python");
        assert_eq!(output, b"blocked\n");
    }
}
