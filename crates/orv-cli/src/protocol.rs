#![allow(clippy::redundant_pub_crate, clippy::wildcard_imports)]

pub(crate) fn read_lsp_content_length<R: std::io::BufRead>(
    reader: &mut R,
) -> anyhow::Result<Option<usize>> {
    let mut content_length = None;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            if saw_header {
                anyhow::bail!("incomplete LSP header");
            }
            return Ok(None);
        }
        let header = line.trim_end_matches('\n').trim_end_matches('\r');
        if header.is_empty() {
            break;
        }
        saw_header = true;
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| anyhow::anyhow!("invalid Content-Length: {e}"))?,
            );
        }
    }
    content_length
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("missing Content-Length header"))
}

pub(crate) fn write_lsp_response_frame<W: std::io::Write>(
    writer: &mut W,
    response: &serde_json::Value,
) -> anyhow::Result<()> {
    let body = serde_json::to_string(response)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    Ok(())
}
