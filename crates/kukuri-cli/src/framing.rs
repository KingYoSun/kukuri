use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};

use crate::protocol::{MAX_FRAME_BYTES, ProtocolError, error_code};

pub async fn read_json_frame<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut output = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(io_error)?;
        if available.is_empty() {
            if output.is_empty() {
                return Ok(None);
            }
            return Err(ProtocolError::new(
                error_code::INVALID_REQUEST,
                "JSON frame ended before a newline",
            ));
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |index| index + 1);
        if output.len().saturating_add(take) > MAX_FRAME_BYTES {
            return Err(ProtocolError::new(
                error_code::INVALID_REQUEST,
                "JSON frame exceeds the supported size",
            ));
        }
        output.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            output.pop();
            if output.last() == Some(&b'\r') {
                output.pop();
            }
            if output.is_empty() {
                return Err(ProtocolError::new(
                    error_code::INVALID_REQUEST,
                    "JSON frame is empty",
                ));
            }
            return Ok(Some(output));
        }
    }
}

pub async fn read_secret_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    length: u64,
) -> Result<Vec<u8>, ProtocolError> {
    let length = usize::try_from(length).map_err(|_| {
        ProtocolError::new(
            error_code::INVALID_REQUEST,
            "secret frame length is invalid",
        )
    })?;
    if length > MAX_FRAME_BYTES {
        return Err(ProtocolError::new(
            error_code::INVALID_REQUEST,
            "secret frame exceeds the supported size",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await.map_err(|_| {
        ProtocolError::new(
            error_code::INVALID_REQUEST,
            "secret frame ended before the declared length",
        )
    })?;
    Ok(bytes)
}

pub async fn write_json_frame<W: AsyncWrite + Unpin, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), ProtocolError> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        ProtocolError::new(error_code::INTERNAL_ERROR, "failed to encode JSON frame")
    })?;
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        return Err(ProtocolError::new(
            error_code::INTERNAL_ERROR,
            "JSON response exceeds the supported size",
        ));
    }
    writer.write_all(&bytes).await.map_err(io_error)?;
    writer.write_all(b"\n").await.map_err(io_error)?;
    writer.flush().await.map_err(io_error)
}

pub async fn write_secret_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
) -> Result<(), ProtocolError> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::new(
            error_code::INTERNAL_ERROR,
            "secret response exceeds the supported size",
        ));
    }
    writer.write_all(bytes).await.map_err(io_error)?;
    writer.flush().await.map_err(io_error)
}

fn io_error(_error: std::io::Error) -> ProtocolError {
    ProtocolError::new(error_code::NETWORK_UNAVAILABLE, "local socket I/O failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn oversized_and_partial_frames_fail_closed() {
        let oversized = vec![b'a'; MAX_FRAME_BYTES + 1];
        let mut reader = tokio::io::BufReader::new(oversized.as_slice());
        assert_eq!(
            read_json_frame(&mut reader)
                .await
                .expect_err("oversized")
                .code,
            error_code::INVALID_REQUEST
        );
        let mut reader = tokio::io::BufReader::new(b"{}".as_slice());
        assert_eq!(
            read_json_frame(&mut reader)
                .await
                .expect_err("partial")
                .code,
            error_code::INVALID_REQUEST
        );
    }
}
