//! Read-only tail of `Client.txt`: only lines written after start, complete lines only.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// A line that never ends would grow the buffer forever; drop it past this size.
const MAX_PARTIAL: usize = 1 << 20;

pub struct Tailer {
    path: PathBuf,
    pos: u64,
    partial: Vec<u8>,
}

impl Tailer {
    /// Starts at the current end of the file, so history is never replayed.
    pub fn new(path: PathBuf) -> Self {
        let pos = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self { path, pos, partial: Vec::new() }
    }

    /// New complete lines since the last call. Err = log unreadable right now (UI shows "can't read log").
    pub fn poll(&mut self) -> std::io::Result<Vec<String>> {
        let len = match std::fs::metadata(&self.path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // rotated away or not created yet: read the next file from its start
                self.pos = 0;
                self.partial.clear();
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };
        if len < self.pos {
            // truncated or replaced by a smaller file
            // shortcut: a replacement file already larger than the old position is not noticed, switch to file-id checks if that shows up
            self.pos = 0;
            self.partial.clear();
        }
        if len == self.pos {
            return Ok(Vec::new());
        }

        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(self.pos))?;
        let mut chunk = Vec::new();
        f.take(len - self.pos).read_to_end(&mut chunk)?;
        self.pos += chunk.len() as u64;

        let at_file_start = self.pos == chunk.len() as u64;
        self.partial.extend_from_slice(&chunk);
        let mut data = std::mem::take(&mut self.partial);
        if at_file_start && data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            data.drain(..3);
        }

        let mut lines = Vec::new();
        let mut start = 0;
        while let Some(i) = data[start..].iter().position(|&b| b == b'\n') {
            let raw = &data[start..start + i];
            let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
            lines.push(String::from_utf8_lossy(raw).into_owned());
            start += i + 1;
        }
        if data.len() - start <= MAX_PARTIAL {
            self.partial = data[start..].to_vec();
        }
        Ok(lines)
    }
}

/// Last complete lines within `max_bytes` of the end: lets the HUD know the current zone/level right after a restart.
pub fn recent_lines(path: &std::path::Path, max_bytes: u64) -> Vec<String> {
    let Ok(mut f) = File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(max_bytes);
    let mut buf = Vec::new();
    if f.seek(SeekFrom::Start(start)).is_err() || f.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.lines().map(str::to_string);
    if start > 0 {
        lines.next(); // cut mid-line
    }
    lines.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::Write;

    fn append(p: &std::path::Path, s: &[u8]) {
        OpenOptions::new().create(true).append(true).open(p).unwrap().write_all(s).unwrap();
    }

    #[test]
    fn skips_history_and_returns_only_new_complete_lines() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        append(&p, b"old line\n");
        let mut tail = Tailer::new(p.clone());
        assert!(tail.poll().unwrap().is_empty());
        append(&p, b"first\r\nsecond\npart");
        assert_eq!(tail.poll().unwrap(), ["first", "second"]);
        append(&p, b"ial\n");
        assert_eq!(tail.poll().unwrap(), ["partial"]);
        assert!(tail.poll().unwrap().is_empty());
    }

    #[test]
    fn restarts_from_the_top_after_truncation() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        append(&p, b"aaaaaaaaaaaaaaaa\n");
        let mut tail = Tailer::new(p.clone());
        std::fs::write(&p, b"new\n").unwrap();
        assert_eq!(tail.poll().unwrap(), ["new"]);
    }

    #[test]
    fn follows_rotation_to_a_new_file() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        append(&p, b"before\n");
        let mut tail = Tailer::new(p.clone());
        std::fs::rename(&p, t.path().join("Client.1.txt")).unwrap();
        assert!(tail.poll().unwrap().is_empty());
        append(&p, b"after rotate\n");
        assert_eq!(tail.poll().unwrap(), ["after rotate"]);
    }

    #[test]
    fn strips_bom_and_survives_invalid_utf8() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        let mut tail = Tailer::new(p.clone()); // file does not exist yet
        append(&p, b"\xEF\xBB\xBFhello\nbad \xFF byte\n");
        assert_eq!(tail.poll().unwrap(), ["hello", "bad \u{FFFD} byte"]);
    }

    #[test]
    fn recent_lines_returns_only_the_tail_without_the_cut_line() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        append(&p, b"aaaaaaaaaa\nbbbbbbbbbb\ncccccccccc\n");
        assert_eq!(recent_lines(&p, 15), ["cccccccccc"]);
        assert_eq!(recent_lines(&p, 1000), ["aaaaaaaaaa", "bbbbbbbbbb", "cccccccccc"]);
        assert!(recent_lines(&t.path().join("none"), 10).is_empty());
    }

    #[test]
    fn huge_unterminated_line_is_dropped_not_buffered_forever() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Client.txt");
        let mut tail = Tailer::new(p.clone());
        append(&p, &vec![b'x'; MAX_PARTIAL + 10]);
        assert!(tail.poll().unwrap().is_empty());
        assert!(tail.partial.is_empty());
    }
}
