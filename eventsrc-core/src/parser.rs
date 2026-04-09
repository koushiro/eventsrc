use bytes::{Buf, Bytes, BytesMut};
use memchr::{memchr, memchr2};

const BOM: &[u8; 3] = b"\xEF\xBB\xBF";
const CF: u8 = b'\r';
const LF: u8 = b'\n';
const COLON: u8 = b':';
const SPACE: u8 = b' ';

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RawLine {
    Empty,
    Comment,
    Field(RawField),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawField {
    line: Bytes,
    name_len: usize,
    value_start: usize,
}

impl RawField {
    #[inline]
    pub(crate) fn name(&self) -> &[u8] {
        &self.line[..self.name_len]
    }

    #[inline]
    pub(crate) fn value(&self) -> &[u8] {
        &self.line[self.value_start..]
    }

    #[inline]
    pub(crate) fn value_bytes(&self) -> Bytes {
        self.line.slice(self.value_start..)
    }
}

#[derive(Debug)]
pub(crate) struct Parser {
    buf: BytesMut,
    leading_bom_pending: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self { buf: BytesMut::default(), leading_bom_pending: true }
    }
}

impl Parser {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);

        if self.leading_bom_pending {
            self.resolve_leading_bom();
        }
    }

    pub(crate) fn next(&mut self) -> Option<RawLine> {
        if self.leading_bom_pending {
            return None;
        }

        let (line_end, remainder_start) = find_line_boundary(self.buf.as_ref())?;
        let line = self.take_line(line_end, remainder_start);
        Some(parse_line(line))
    }

    fn resolve_leading_bom(&mut self) {
        match starts_with_bom(self.buf.as_ref()) {
            Some(true) => {
                self.buf.advance(BOM.len());
                self.leading_bom_pending = false;
            },
            Some(false) => {
                self.leading_bom_pending = false;
            },
            None => {},
        }
    }

    fn take_line(&mut self, line_end: usize, remainder_start: usize) -> Bytes {
        let line = self.buf.split_to(line_end).freeze();
        self.buf.advance(remainder_start - line_end);
        line
    }
}

fn starts_with_bom(bytes: &[u8]) -> Option<bool> {
    match bytes.len() {
        0 => None,
        1 if bytes[0] == BOM[0] => None,
        2 if bytes[..2] == BOM[..2] => None,
        _ => Some(bytes.starts_with(BOM)),
    }
}

fn find_line_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let line_end = memchr2(CF, LF, bytes)?;
    let remainder_start = match bytes[line_end] {
        LF => line_end + 1,
        CF => match bytes.get(line_end + 1) {
            Some(&LF) => line_end + 2,
            Some(_) => line_end + 1,
            None => return None,
        },
        _ => unreachable!(),
    };

    Some((line_end, remainder_start))
}

// Parse a fully delimited SSE line:
// - `:...` becomes a comment
// - `field: value` stores a raw field view over the original line
// - `field` is treated as an empty value
fn parse_line(line: Bytes) -> RawLine {
    if line.is_empty() {
        return RawLine::Empty;
    }

    match memchr(COLON, line.as_ref()) {
        Some(0) => RawLine::Comment,
        Some(pos) => {
            let value_start = if line.get(pos + 1) == Some(&SPACE) { pos + 2 } else { pos + 1 };
            RawLine::Field(RawField { line, name_len: pos, value_start })
        },
        None => {
            let len = line.len();
            RawLine::Field(RawField { line, name_len: len, value_start: len })
        },
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use super::*;

    impl RawLine {
        pub(crate) fn field(name: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Self {
            let name = name.as_ref();
            let value = value.as_ref();

            let (line, value_start) = if value.is_empty() {
                let mut line = Vec::with_capacity(name.len() + 1);
                line.extend_from_slice(name);
                line.push(b':');
                (line, name.len() + 1)
            } else {
                let mut line = Vec::with_capacity(name.len() + 2 + value.len());
                line.extend_from_slice(name);
                line.extend_from_slice(b": ");
                line.extend_from_slice(value);
                (line, name.len() + 2)
            };

            Self::Field(RawField {
                line: Bytes::copy_from_slice(&line),
                name_len: name.len(),
                value_start,
            })
        }

        pub(crate) fn raw_field(
            raw: impl AsRef<[u8]>,
            name: impl AsRef<[u8]>,
            value: impl AsRef<[u8]>,
        ) -> Self {
            let raw = raw.as_ref();
            let name = name.as_ref();
            let value = value.as_ref();

            Self::Field(RawField {
                line: Bytes::copy_from_slice(raw),
                name_len: name.len(),
                value_start: raw.len() - value.len(),
            })
        }
    }

    fn parse_chunk(chunk: impl AsRef<[u8]>) -> Vec<RawLine> {
        let mut parser = Parser::new();
        parser.push(chunk.as_ref());
        collect_lines(&mut parser)
    }

    fn collect_lines(parser: &mut Parser) -> Vec<RawLine> {
        let mut lines = Vec::new();

        while let Some(line) = parser.next() {
            lines.push(line);
        }

        lines
    }

    #[test]
    fn parses_blank_line_as_empty() {
        assert_eq!(parse_chunk(b"\n"), vec![RawLine::Empty]);
    }

    #[test]
    fn parses_comment_line_starting_with_colon() {
        assert_eq!(parse_chunk(b": keepalive\n"), vec![RawLine::Comment]);
    }

    #[test]
    fn parses_field_without_colon_as_empty_value() {
        assert_eq!(parse_chunk(b"event\n"), vec![RawLine::raw_field("event", "event", "")]);
    }

    #[test]
    fn parses_field_with_empty_value_after_colon() {
        assert_eq!(parse_chunk(b"event:\n"), vec![RawLine::field("event", "")]);
    }

    #[test]
    fn strips_one_optional_space_after_colon() {
        assert_eq!(parse_chunk(b"data: hello\n"), vec![RawLine::field("data", "hello")]);
    }

    #[test]
    fn preserves_additional_space_after_optional_space() {
        assert_eq!(
            parse_chunk(b"data:  hello\n"),
            vec![RawLine::raw_field("data:  hello", "data", " hello")]
        );
    }

    #[test]
    fn preserves_bytes_without_utf8_validation() {
        assert_eq!(
            parse_chunk(b"\xff: \xfe\n"),
            vec![RawLine::raw_field(b"\xff: \xfe", b"\xff", b"\xfe")]
        );
    }

    #[test]
    fn parses_multiple_complete_lines_in_order() {
        assert_eq!(
            parse_chunk(b": keepalive\n\ndata: hello\n"),
            vec![RawLine::Comment, RawLine::Empty, RawLine::field("data", "hello")]
        );
    }

    #[test]
    fn parses_field_lines_incrementally_across_chunks() {
        let mut parser = Parser::new();

        parser.push(b"da");
        assert!(parser.next().is_none());

        parser.push(b"ta: hello\n");
        assert_eq!(parser.next(), Some(RawLine::field("data", "hello")));
    }

    #[test]
    fn accepts_lf_crlf_and_lone_cr_as_line_endings() {
        let mut parser = Parser::new();

        parser.push(b"data: lf\ndata: crlf\r\ndata: lone-cr\rx");
        assert_eq!(parser.next(), Some(RawLine::field("data", "lf")));
        assert_eq!(parser.next(), Some(RawLine::field("data", "crlf")));
        assert_eq!(parser.next(), Some(RawLine::field("data", "lone-cr")));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn eof_does_not_complete_partial_lines() {
        let mut parser = Parser::new();

        parser.push(b"data: payload");

        assert_eq!(parser.next(), None);
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn waits_for_full_bom_prefix_before_parsing() {
        let mut parser = Parser::new();

        parser.push(&BOM[..2]);
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn strips_bom_once_across_chunks() {
        let mut parser = Parser::new();

        parser.push(&BOM[..2]);
        let mut chunk = BOM[2..].to_vec();
        chunk.extend_from_slice(b"data: hello\n");
        parser.push(&chunk);

        assert_eq!(parser.next(), Some(RawLine::field("data", "hello")));
    }

    #[test]
    fn does_not_strip_bom_after_startup_has_completed() {
        let mut parser = Parser::new();

        parser.push(b"data: one\n");
        assert_eq!(parser.next(), Some(RawLine::field("data", "one")));

        let mut chunk = BOM.to_vec();
        chunk.extend_from_slice(b"data: two\n");
        parser.push(&chunk);

        assert_eq!(
            parser.next(),
            Some(RawLine::raw_field(b"\xEF\xBB\xBFdata: two", b"\xEF\xBB\xBFdata", "two"))
        );
    }
}
