//! The header at the top of a Markdown file.
//!
//! It is the `---` block that other writing tools call front matter, in a
//! small part of YAML: each line is `key: value`, and a value is a piece of
//! text or a list of pieces of text. Keys this program does not know are
//! kept and written back.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Text(String),
    List(Vec<String>),
}

/// The entries of a header, in the order they appear in the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub entries: Vec<(String, Value)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderError {
    /// Line in the file, counted from 1.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for HeaderError {}

impl Header {
    pub fn text(&self, key: &str) -> Option<&str> {
        self.entries.iter().find_map(|(name, value)| match value {
            Value::Text(text) if name == key => Some(text.as_str()),
            _ => None,
        })
    }

    pub fn list(&self, key: &str) -> Option<&[String]> {
        self.entries.iter().find_map(|(name, value)| match value {
            Value::List(items) if name == key => Some(items.as_slice()),
            _ => None,
        })
    }

    /// Set a text entry. An empty value removes the entry: a key that is
    /// missing and a key that is empty mean the same.
    pub fn set_text(&mut self, key: &str, value: &str) {
        self.set(key, (!value.is_empty()).then(|| Value::Text(value.to_string())));
    }

    pub fn set_list(&mut self, key: &str, items: &[String]) {
        self.set(key, (!items.is_empty()).then(|| Value::List(items.to_vec())));
    }

    fn set(&mut self, key: &str, value: Option<Value>) {
        let at = self.entries.iter().position(|(name, _)| name == key);
        match (at, value) {
            (Some(at), Some(value)) => self.entries[at].1 = value,
            (Some(at), None) => {
                self.entries.remove(at);
            }
            (None, Some(value)) => self.entries.push((key.to_string(), value)),
            (None, None) => {}
        }
    }

    /// Take an entry out, for a reader that then keeps what is left as the
    /// keys it does not know.
    pub fn take_text(&mut self, key: &str) -> String {
        let at = self
            .entries
            .iter()
            .position(|(name, value)| name == key && matches!(value, Value::Text(_)));
        match at.map(|at| self.entries.remove(at)) {
            Some((_, Value::Text(text))) => text,
            _ => String::new(),
        }
    }

    /// The header as it is written in a file, from `---` to `---`.
    pub fn to_text(&self) -> String {
        let mut out = String::from("---\n");
        for (key, value) in &self.entries {
            match value {
                Value::Text(text) => write_scalar(&mut out, key, text),
                Value::List(items) => {
                    out.push_str(key);
                    out.push_str(":\n");
                    for item in items {
                        out.push_str("  - ");
                        out.push_str(&inline_scalar(item));
                        out.push('\n');
                    }
                }
            }
        }
        out.push_str("---\n");
        out
    }
}

/// Split a file into its header and the text after it. A file that does not
/// start with `---` has no header, and all of it is body.
pub fn split_header(file: &str) -> Result<(Header, &str), HeaderError> {
    let file = file.strip_prefix('\u{feff}').unwrap_or(file);
    let mut lines = LineCursor { rest: file, number: 0 };
    match lines.peek() {
        Some(first) if first.trim_end() == "---" => {
            lines.next();
        }
        _ => return Ok((Header::default(), file)),
    }
    let mut header = Header::default();
    loop {
        let Some(line) = lines.next() else {
            return Err(HeaderError {
                line: lines.number,
                message: "the header is not closed with ---".into(),
            });
        };
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            break;
        }
        if trimmed.trim_start().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let number = lines.number;
        let error = |message: &str| HeaderError {
            line: number,
            message: message.into(),
        };
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(error("expected `key: value`"));
        };
        let valid_key = !key.is_empty()
            && key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
        if !valid_key || !(value.is_empty() || value.starts_with(' ')) {
            return Err(error("expected `key: value`"));
        }
        let value = value.trim();
        let entry = if value.is_empty() {
            let mut items = Vec::new();
            while let Some(next) = lines.peek() {
                let Some(item) = next.trim_start().strip_prefix('-') else {
                    break;
                };
                if !(item.is_empty() || item.starts_with(' ')) {
                    break;
                }
                lines.next();
                items.push(read_scalar(item.trim()).map_err(|message| HeaderError {
                    line: lines.number,
                    message,
                })?);
            }
            if items.is_empty() {
                continue;
            }
            Value::List(items)
        } else if matches!(value, "|" | "|-" | "|+") {
            Value::Text(read_block(&mut lines, value))
        } else {
            Value::Text(read_scalar(value).map_err(|message| error(&message))?)
        };
        header.entries.retain(|(name, _)| name != key);
        header.entries.push((key.to_string(), entry));
    }
    let body = lines.rest;
    Ok((header, body.strip_prefix('\n').unwrap_or(body)))
}

struct LineCursor<'a> {
    rest: &'a str,
    number: usize,
}

impl<'a> LineCursor<'a> {
    fn peek(&self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        let line = self.rest.split('\n').next().unwrap_or("");
        Some(line.strip_suffix('\r').unwrap_or(line))
    }

    fn next(&mut self) -> Option<&'a str> {
        let line = self.peek()?;
        let raw = self.rest.split('\n').next().unwrap_or("");
        self.rest = self.rest[raw.len()..].strip_prefix('\n').unwrap_or("");
        self.number += 1;
        Some(line)
    }
}

/// Lines under `key: |`. They are indented; the indent of the first one is
/// taken off all of them.
fn read_block(lines: &mut LineCursor, style: &str) -> String {
    let mut body: Vec<&str> = Vec::new();
    let mut indent: Option<usize> = None;
    while let Some(line) = lines.peek() {
        let depth = line.len() - line.trim_start_matches(' ').len();
        if !line.trim().is_empty() && depth == 0 {
            break;
        }
        lines.next();
        if line.trim().is_empty() {
            body.push("");
            continue;
        }
        let indent = *indent.get_or_insert(depth);
        body.push(&line[indent.min(depth)..]);
    }
    let trailing = body.iter().rev().take_while(|line| line.is_empty()).count();
    let kept = body.len() - trailing;
    let mut text = body[..kept].join("\n");
    match style {
        "|" if kept > 0 => text.push('\n'),
        "|+" => text.push_str(&"\n".repeat(trailing + usize::from(kept > 0))),
        _ => {}
    }
    text
}

fn read_scalar(value: &str) -> Result<String, String> {
    if let Some(rest) = value.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = rest.chars();
        loop {
            match chars.next() {
                None => return Err("a quoted value is not closed".into()),
                Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('0') => out.push('\0'),
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        let ch = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
                        out.push(ch.ok_or("a \\u escape is not a character")?);
                    }
                    Some(other) => out.push(other),
                    None => return Err("a quoted value is not closed".into()),
                },
                Some(ch) => out.push(ch),
            }
        }
        return Ok(out);
    }
    if let Some(rest) = value.strip_prefix('\'') {
        let Some(inner) = rest.strip_suffix('\'') else {
            return Err("a quoted value is not closed".into());
        };
        return Ok(inner.replace("''", "'"));
    }
    // In YAML, ` #` starts a comment in an unquoted value.
    let value = value.split(" #").next().unwrap_or(value);
    Ok(value.trim_end().to_string())
}

/// A value can be written bare when no YAML reader would take it for
/// anything but that text.
fn is_plain(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    let reserved = [
        "true", "false", "yes", "no", "on", "off", "null", "y", "n",
    ];
    first.is_alphanumeric()
        && !value.ends_with(|ch: char| ch.is_whitespace() || ch == ':')
        && !value.contains(": ")
        && !value.contains(" #")
        && !value.chars().any(char::is_control)
        && value.parse::<f64>().is_err()
        && !reserved.contains(&value.to_ascii_lowercase().as_str())
}

fn inline_scalar(value: &str) -> String {
    if is_plain(value) {
        return value.to_string();
    }
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Text of several lines is written as an indented block when that can be
/// read back exactly, so that a synopsis reads as it was typed.
fn write_scalar(out: &mut String, key: &str, value: &str) {
    let lines: Vec<&str> = value.split('\n').collect();
    let block = lines.len() > 1
        && !value.chars().any(|ch| ch.is_control() && ch != '\n')
        && !lines[0].starts_with(' ')
        && !lines[0].is_empty()
        && !lines[lines.len() - 1].is_empty()
        && lines
            .iter()
            .all(|line| !line.starts_with(' ') && !line.ends_with(' '));
    if !block {
        out.push_str(&format!("{key}: {}\n", inline_scalar(value)));
        return;
    }
    out.push_str(&format!("{key}: |-\n"));
    for line in lines {
        if !line.is_empty() {
            out.push_str("  ");
            out.push_str(line);
        }
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(header: &Header) -> String {
        let text = header.to_text();
        let file = format!("{text}\nbody");
        let (read, body) = split_header(&file).unwrap();
        assert_eq!(&read, header, "file was:\n{text}");
        assert_eq!(body, "body");
        text
    }

    #[test]
    fn writes_plain_values_bare_and_the_rest_quoted() {
        let mut header = Header::default();
        header.set_text("title", "The Harbor");
        header.set_text("status", "draft");
        header.set_text("goal", "Reach the boat: before the tide");
        header.set_text("label", "1984");
        header.set_text("color", "");
        header.set_text("synopsis", "Mara runs.\n\nThe boat has gone.");
        header.set_list("also", &["Mara Vance".into(), "\"Rose\"".into()]);
        assert_eq!(
            round_trip(&header),
            concat!(
                "---\n",
                "title: The Harbor\n",
                "status: draft\n",
                "goal: \"Reach the boat: before the tide\"\n",
                "label: \"1984\"\n",
                "synopsis: |-\n  Mara runs.\n\n  The boat has gone.\n",
                "also:\n  - Mara Vance\n  - \"\\\"Rose\\\"\"\n",
                "---\n",
            )
        );
        assert!(header.text("color").is_none());
        assert_eq!(header.list("also").map(<[String]>::len), Some(2));
    }

    #[test]
    fn every_awkward_value_reads_back_the_same() {
        for value in [
            "true", "No", "~", "- dash", "#tag", " lead", "trail ", "ends:", "a #b", "'single'",
            "\"double\"", "tab\there", "line\n", "\nline", "a\n b", "a \nb", "a\r\nb", "é and —",
            "x\\y", "[list]", "{map}", "*star", "&anchor", "@at", "`tick", "|", ">", "3.5", "0x1F",
            "a\n\n\nb", "a\u{1}b",
        ] {
            let mut header = Header::default();
            header.set_text("key", value);
            header.set_list("items", &[value.to_string(), "plain".into()]);
            round_trip(&header);
        }
    }

    #[test]
    fn reads_what_a_person_might_type() {
        let file = concat!(
            "\u{feff}---\r\n",
            "# a comment\n",
            "title: 'It''s late'   \n",
            "status: draft # not final\n",
            "empty:\n",
            "tags:\n- one\n  - \"two\"\n",
            "notes: |\n    kept\n      indented\n\n",
            "title: Later wins\n",
            "---\n",
            "\n",
            "First line.\n",
        );
        let (header, body) = split_header(file).unwrap();
        assert_eq!(header.text("title"), Some("Later wins"));
        assert_eq!(header.text("status"), Some("draft"));
        assert!(header.text("empty").is_none());
        assert_eq!(header.list("tags").unwrap(), ["one", "two"]);
        assert_eq!(header.text("notes"), Some("kept\n  indented\n"));
        assert_eq!(body, "First line.\n");
    }

    #[test]
    fn a_file_without_a_header_is_all_body_and_a_broken_one_names_its_line() {
        let (header, body) = split_header("Just text.\n---\n").unwrap();
        assert!(header.entries.is_empty());
        assert_eq!(body, "Just text.\n---\n");

        let unclosed = split_header("---\ntitle: x\n").unwrap_err();
        assert_eq!(unclosed.line, 2);
        let broken = split_header("---\ntitle: x\nnot a pair\n---\n").unwrap_err();
        assert_eq!(broken.line, 3);
        assert_eq!(broken.to_string(), "line 3: expected `key: value`");
    }
}
