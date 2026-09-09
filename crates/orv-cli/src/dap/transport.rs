#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

use super::*;

pub(crate) fn cmd_dap_serve(use_stdio: bool) -> anyhow::Result<()> {
    if !use_stdio {
        anyhow::bail!("dap serve currently requires --stdio");
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    dap_serve_stdio_stream(&mut reader, &mut writer)
}

#[cfg(test)]
pub(crate) fn dap_protocol_response(request: &serde_json::Value) -> serde_json::Value {
    DapSession::default()
        .message_response(request)
        .expect("DAP response")
}

pub(crate) fn dap_serve_stdio_stream<R, W>(reader: &mut R, writer: &mut W) -> anyhow::Result<()>
where
    R: std::io::BufRead,
    W: std::io::Write,
{
    let mut session = DapSession::default();
    loop {
        let Some(content_length) = read_lsp_content_length(reader)? else {
            return Ok(());
        };
        let mut body = vec![0_u8; content_length];
        std::io::Read::read_exact(reader, &mut body)?;
        let request: serde_json::Value = serde_json::from_slice(&body)?;
        if let Some(response) = session.message_response(&request) {
            write_lsp_response_frame(writer, &response)?;
            for event in session.drain_pending_events() {
                write_lsp_response_frame(writer, &event)?;
            }
            writer.flush()?;
        }
    }
}

#[cfg(test)]
pub(crate) fn dap_stdio_response(input: &str) -> anyhow::Result<String> {
    let mut reader = std::io::Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    dap_serve_stdio_stream(&mut reader, &mut writer)?;
    String::from_utf8(writer).map_err(|e| anyhow::anyhow!("invalid utf-8 DAP response: {e}"))
}

pub(crate) fn dap_protocol_input_frames(requests: &[serde_json::Value]) -> anyhow::Result<String> {
    let mut input = String::new();
    for request in requests {
        let body = serde_json::to_string(request)?;
        write!(&mut input, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    }
    Ok(input)
}

pub(crate) fn dap_protocol_output_frames(output: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut reader = std::io::Cursor::new(output.as_bytes());
    let mut frames = Vec::new();
    loop {
        let Some(content_length) = read_lsp_content_length(&mut reader)? else {
            return Ok(frames);
        };
        let mut body = vec![0_u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut body)?;
        frames.push(serde_json::from_slice(&body)?);
    }
}
