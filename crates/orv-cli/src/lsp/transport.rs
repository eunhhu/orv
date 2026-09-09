#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn lsp_serve_stdio_stream<R, W>(reader: &mut R, writer: &mut W) -> anyhow::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
{
    let mut session = LspSession::default();
    loop {
        let Some(content_length) = read_lsp_content_length(reader)? else {
            return Ok(());
        };
        let mut body = vec![0_u8; content_length];
        std::io::Read::read_exact(reader, &mut body)?;
        let request: serde_json::Value = serde_json::from_slice(&body)?;
        if let Some(response) = session.message_response(&request) {
            write_lsp_response_frame(writer, &response)?;
            writer.flush()?;
        }
    }
}

#[cfg(test)]
pub(crate) fn lsp_stdio_response(input: &str) -> anyhow::Result<String> {
    let mut reader = std::io::Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    lsp_serve_stdio_stream(&mut reader, &mut writer)?;
    String::from_utf8(writer).map_err(|e| anyhow::anyhow!("invalid utf-8 LSP response: {e}"))
}
