use crate::failure::SCRATCH_BUSY;
use kukuri_cn_safety::ScanError;
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};
use tempfile::TempDir;

/// Maintenance holds the root lock only for cleanup and mkdir. A forked child of any thread
/// also keeps a just-closed lock until its exec, so a short bounded wait is expected.
const MAINTENANCE_WAIT: Duration = Duration::from_secs(2);
const MAINTENANCE_POLL: Duration = Duration::from_millis(10);

pub struct JobDirectory {
    directory: TempDir,
    _lease: File,
}

impl JobDirectory {
    /// Startup check; blocks the caller for at most `MAINTENANCE_WAIT`.
    pub fn create_blocking(root: &Path) -> Result<Self, ScanError> {
        let deadline = std::time::Instant::now() + MAINTENANCE_WAIT;
        loop {
            match Self::try_create(root)? {
                Some(directory) => return Ok(directory),
                None if std::time::Instant::now() < deadline => {
                    std::thread::sleep(MAINTENANCE_POLL)
                }
                None => return Err(unavailable(SCRATCH_BUSY)),
            }
        }
    }

    pub async fn create(root: &Path) -> Result<Self, ScanError> {
        let deadline = tokio::time::Instant::now() + MAINTENANCE_WAIT;
        loop {
            match Self::try_create(root)? {
                Some(directory) => return Ok(directory),
                None if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(MAINTENANCE_POLL).await
                }
                None => return Err(unavailable(SCRATCH_BUSY)),
            }
        }
    }

    /// `Ok(None)` means another holder is maintaining the root.
    fn try_create(root: &Path) -> Result<Option<Self>, ScanError> {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(root)
            .map_err(|_| unavailable("cannot create video tmpfs directory"))?;
        crate::sandbox::require_tmpfs(root)
            .map_err(|_| unavailable("video scratch directory is not a private tmpfs"))?;
        let root_lock = open_lock(&root.join(".kukuri-video-root-lock"))?;
        match root_lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(std::fs::TryLockError::Error(_)) => {
                return Err(unavailable("cannot open video job lease"));
            }
        }
        // Only this extractor's private, non-symlink directories are candidates.
        // A live job holds its lease until its children have been reaped.
        for entry in std::fs::read_dir(root)
            .map_err(|_| unavailable("cannot inspect video scratch directory"))?
        {
            let entry = entry.map_err(|_| unavailable("cannot inspect video scratch entry"))?;
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with(".kukuri-video-job-")
                || !entry
                    .file_type()
                    .map_err(|_| unavailable("cannot inspect video scratch type"))?
                    .is_dir()
            {
                continue;
            }
            let lease = open_lock(&entry.path().join(".lease"))?;
            if lease.try_lock().is_ok() {
                std::fs::remove_dir_all(entry.path())
                    .map_err(|_| unavailable("cannot clean abandoned video job"))?;
            }
        }
        let directory = tempfile::Builder::new()
            .prefix(".kukuri-video-job-")
            .tempdir_in(root)
            .map_err(|_| unavailable("cannot create video job directory"))?;
        let lease = open_lock(&directory.path().join(".lease"))?;
        lease
            .try_lock()
            .map_err(|_| unavailable("cannot lock video job directory"))?;
        Ok(Some(Self {
            directory,
            _lease: lease,
        }))
    }
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
}

fn open_lock(path: &Path) -> Result<File, ScanError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|_| unavailable("cannot open video job lease"))
}

fn unavailable(reason: &str) -> ScanError {
    ScanError::Unavailable(reason.to_owned())
}
