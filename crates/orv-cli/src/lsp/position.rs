//! LSP defaults to UTF-16; source spans remain UTF-8 byte offsets internally.

/// Convert a source byte offset into a zero-based UTF-16 LSP position.
pub(crate) fn lsp_byte_position(source: &str, byte: u32) -> (usize, usize) {
    let mut byte = usize::try_from(byte)
        .unwrap_or(source.len())
        .min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    let prefix = &source[..byte];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, tail)| tail)
        .trim_end_matches('\r')
        .encode_utf16()
        .count();
    (line, character)
}

/// Convert a zero-based UTF-16 LSP position into a source byte offset.
pub(crate) fn lsp_position_to_byte(source: &str, position: (usize, usize)) -> usize {
    let (target_line, target_character) = position;
    let mut line = 0;
    let mut character = 0;
    for (byte, ch) in source.char_indices() {
        if line == target_line
            && (character >= target_character
                || matches!(ch, '\r' | '\n')
                || character + ch.len_utf16() > target_character)
        {
            return byte;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16();
        }
    }
    source.len()
}

#[cfg(test)]
mod tests {
    use super::{lsp_byte_position, lsp_position_to_byte};

    #[test]
    fn unicode_positions_clamp_to_crlf_line_ends_and_scalar_boundaries() {
        let source = "가😀z\r\nnext\r\n";
        assert_eq!(lsp_byte_position(source, 7), (0, 3));
        assert_eq!(lsp_byte_position(source, 8), (0, 4));
        assert_eq!(lsp_byte_position(source, 9), (0, 4));
        assert_eq!(lsp_byte_position(source, 10), (1, 0));
        assert_eq!(lsp_position_to_byte(source, (0, 1)), 3);
        assert_eq!(lsp_position_to_byte(source, (0, 2)), 3);
        assert_eq!(lsp_position_to_byte(source, (0, 3)), 7);
        assert_eq!(lsp_position_to_byte(source, (0, 100)), 8);
        assert_eq!(lsp_position_to_byte(source, (1, 0)), 10);
        assert_eq!(lsp_position_to_byte(source, (20, 0)), source.len());
    }
}
