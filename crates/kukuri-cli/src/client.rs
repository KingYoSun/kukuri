use std::os::unix::ffi::OsStrExt;
use std::{ffi::OsStr, path::PathBuf, time::Duration};

use kukuri_desktop_runtime::ClientProfile;
use tokio::{
    io::{AsyncWriteExt, BufReader, ReadHalf, WriteHalf},
    net::UnixStream,
};

use crate::{
    framing::{read_json_frame, read_secret_frame, write_json_frame, write_secret_frame},
    protocol::{
        ProtocolError, RequestEnvelope, ResponseEnvelope, SCHEMA_VERSION, SecretInput,
        SecretOutput, error_code,
    },
};

pub struct ClientSession {
    reader: BufReader<ReadHalf<UnixStream>>,
    _writer: WriteHalf<UnixStream>,
    request_id: String,
    command: String,
    profile: String,
    accepts_secret_output: bool,
}

impl ClientSession {
    pub async fn connect(
        profile: &ClientProfile,
        request: &RequestEnvelope,
        secret: Option<&SecretInput>,
    ) -> Result<Self, ProtocolError> {
        let socket_path = daemon_socket_path(profile)?;
        let timeout = Duration::from_millis(request.timeout_ms()?);
        let deadline = tokio::time::Instant::now() + timeout;
        let stream = tokio::time::timeout(timeout, UnixStream::connect(&socket_path))
            .await
            .map_err(|_| ProtocolError::new(error_code::TIMEOUT, "daemon connection timed out"))?
            .map_err(|_| {
                ProtocolError::new(error_code::DAEMON_UNAVAILABLE, "daemon is unavailable")
            })?;
        let (read, mut write) = tokio::io::split(stream);
        let mut reader = BufReader::new(read);
        tokio::time::timeout_at(deadline, write_json_frame(&mut write, request))
            .await
            .map_err(|_| ProtocolError::new(error_code::TIMEOUT, "request output timed out"))??;
        if let Some(secret) = secret {
            let bytes = tokio::time::timeout_at(deadline, read_json_frame(&mut reader))
                .await
                .map_err(|_| {
                    ProtocolError::new(error_code::TIMEOUT, "secret preflight timed out")
                })??
                .ok_or_else(|| {
                    ProtocolError::new(
                        error_code::NETWORK_UNAVAILABLE,
                        "daemon closed during secret preflight",
                    )
                })?;
            let response: ResponseEnvelope = serde_json::from_slice(&bytes).map_err(|_| {
                ProtocolError::new(error_code::INVALID_REQUEST, "daemon returned invalid JSON")
            })?;
            validate_response(request, &response)?;
            if !response.ok {
                return Err(response_error(response));
            }
            if !response.more
                || response
                    .data
                    .as_ref()
                    .and_then(|data| data.get("ready_for_secret"))
                    != Some(&serde_json::Value::Bool(true))
            {
                return Err(ProtocolError::new(
                    error_code::PROTOCOL_MISMATCH,
                    "daemon did not authorize the secret frame",
                ));
            }
            tokio::time::timeout_at(deadline, write_secret_frame(&mut write, secret.expose()))
                .await
                .map_err(|_| ProtocolError::new(error_code::TIMEOUT, "secret input timed out"))??;
        }
        tokio::time::timeout_at(deadline, async {
            write.shutdown().await.map_err(|_| {
                ProtocolError::new(error_code::NETWORK_UNAVAILABLE, "failed to finish request")
            })
        })
        .await
        .map_err(|_| ProtocolError::new(error_code::TIMEOUT, "request output timed out"))??;
        Ok(Self {
            reader,
            _writer: write,
            request_id: request.request_id.clone(),
            command: request.command.clone(),
            profile: request.profile.clone(),
            accepts_secret_output: request.accepts_secret_output,
        })
    }

    pub async fn next(
        &mut self,
    ) -> Result<Option<(ResponseEnvelope, Option<SecretOutput>)>, ProtocolError> {
        let Some(bytes) = read_json_frame(&mut self.reader).await? else {
            return Ok(None);
        };
        let response: ResponseEnvelope = serde_json::from_slice(&bytes).map_err(|_| {
            ProtocolError::new(error_code::INVALID_REQUEST, "daemon returned invalid JSON")
        })?;
        validate_response_parts(&self.request_id, &self.command, &self.profile, &response)?;
        if response.secret_bytes.is_some() && !self.accepts_secret_output {
            return Err(ProtocolError::new(
                error_code::PROTOCOL_MISMATCH,
                "daemon returned an undeclared secret frame",
            ));
        }
        let secret = match response.secret_bytes {
            Some(length) => Some(SecretOutput::new(
                read_secret_frame(&mut self.reader, length).await?,
            )),
            None => None,
        };
        Ok(Some((response, secret)))
    }
}

fn validate_response(
    request: &RequestEnvelope,
    response: &ResponseEnvelope,
) -> Result<(), ProtocolError> {
    validate_response_parts(
        &request.request_id,
        &request.command,
        &request.profile,
        response,
    )
}

fn validate_response_parts(
    request_id: &str,
    command: &str,
    profile: &str,
    response: &ResponseEnvelope,
) -> Result<(), ProtocolError> {
    if response.schema_version != SCHEMA_VERSION
        || response.request_id != request_id
        || response.command != command
        || response.profile != profile
    {
        return Err(ProtocolError::new(
            error_code::PROTOCOL_MISMATCH,
            "daemon response does not match the request",
        ));
    }
    Ok(())
}

fn response_error(response: ResponseEnvelope) -> ProtocolError {
    match response.error {
        Some(error) => ProtocolError {
            code: error.code,
            message: error.message,
            details: error.details,
        },
        None => ProtocolError::new(error_code::INTERNAL_ERROR, "daemon returned an empty error"),
    }
}

pub fn daemon_socket_path(profile: &ClientProfile) -> Result<PathBuf, ProtocolError> {
    let runtime_root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            ProtocolError::new(
                error_code::DAEMON_UNAVAILABLE,
                "XDG_RUNTIME_DIR is required for the daemon socket",
            )
        })?;
    let profile_path = std::fs::canonicalize(&profile.app_data_dir).map_err(|_| {
        ProtocolError::new(
            error_code::DAEMON_UNAVAILABLE,
            "profile directory is unavailable",
        )
    })?;
    let digest = blake3::hash(os_str_bytes(profile_path.as_os_str())).to_hex();
    Ok(runtime_root
        .join("kukuri")
        .join(format!("profile-{}.sock", &digest[..32])))
}

fn os_str_bytes(value: &OsStr) -> &[u8] {
    value.as_bytes()
}
