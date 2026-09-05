use std::{fs::OpenOptions, io::Write, path::PathBuf};

use base64::Engine;
use serde_json::{Value, json};

use super::{command_error, decode, runtime, schema};
use crate::{
    protocol::{ProtocolError, error_code},
    registry::{CommandOutput, HandlerContext},
};

pub(super) async fn export(
    context: &HandlerContext<'_>,
    mut payload: Value,
    preview: bool,
) -> Result<CommandOutput, ProtocolError> {
    let output_path = payload
        .as_object_mut()
        .and_then(|fields| fields.remove("output_path"))
        .and_then(|value| value.as_str().map(PathBuf::from))
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            ProtocolError::new(
                error_code::VALIDATION_FAILED,
                "output_pathは絶対pathで指定してください",
            )
        })?;
    let runtime = runtime(context)?;
    let media = if preview {
        let result = runtime
            .get_blob_preview_url(decode(payload)?)
            .await
            .map_err(command_error)?;
        result
            .map(|url| {
                // 共有runtimeのdata URLだけを解析する。URLをpathや取得先として実行しない。
                let (header, data) = url.split_once(",").ok_or_else(invalid_media)?;
                let mime = header
                    .strip_prefix("data:")
                    .and_then(|header| header.strip_suffix(";base64"))
                    .ok_or_else(invalid_media)?;
                Ok((mime.to_owned(), data.to_owned()))
            })
            .transpose()?
    } else {
        runtime
            .get_blob_media_payload(decode(payload)?)
            .await
            .map_err(command_error)?
            .map(|value| (value.mime, value.bytes_base64))
    };
    let Some((mime, encoded)) = media else {
        return Ok(CommandOutput::Unary(Value::Null));
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| invalid_media())?;
    let result = tokio::task::spawn_blocking(move || write_output(output_path, mime, bytes))
        .await
        .map_err(|_| invalid_media())??;
    Ok(CommandOutput::Unary(result))
}

fn invalid_media() -> ProtocolError {
    ProtocolError::new(error_code::INTERNAL_ERROR, "メディアを出力できませんでした")
}

fn write_output(path: PathBuf, mime: String, bytes: Vec<u8>) -> Result<Value, ProtocolError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            ProtocolError::new(
                error_code::CONFLICT,
                "出力先が既に存在します。別のpathを指定してください",
            )
        } else {
            invalid_media()
        }
    })?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| invalid_media())?;
    Ok(
        json!({"path": path, "mime": mime, "byte_size": bytes.len(), "hash": blake3::hash(&bytes).to_hex().to_string()}),
    )
}

pub(super) fn output_schema() -> Value {
    schema::nullable(schema::object(
        json!({"path": {"type": "string"}, "mime": {"type": "string"},
        "byte_size": {"type": "integer", "minimum": 0}, "hash": {"type": "string"}}),
        &["path", "mime", "byte_size", "hash"],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_output_does_not_overwrite_files_or_embed_media_in_json() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("media");
        let content = b"private-media-sentinel";
        let result = write_output(path.clone(), "image/png".into(), content.to_vec()).unwrap();
        assert_eq!(result["hash"], blake3::hash(content).to_hex().to_string());
        assert_eq!(std::fs::read(&path).unwrap(), content);
        assert!(!result.to_string().contains("sentinel"));
        assert!(result.get("data_base64").is_none());
        assert_eq!(
            write_output(path.clone(), "image/png".into(), b"replacement".to_vec())
                .unwrap_err()
                .code,
            error_code::CONFLICT
        );
        assert_eq!(std::fs::read(&path).unwrap(), content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
