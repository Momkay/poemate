//! Gets PoB XML from what the user gives us: an import code, a pobb.in / pastebin link, or a local file.
//! Network only runs when the user triggers an import, only to the two known hosts, with size and time caps.

use std::io::Read;
use std::time::Duration;

use crate::pob::{self, DecodeError};

const MAX_DOWNLOAD: u64 = 2 << 20;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    Code(String),
    Url(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImportError {
    Empty,
    UnsupportedLink,
    Offline,
    NotFound,
    BadResponse,
    TooLarge,
    Decode(DecodeError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "Nothing to import",
            Self::UnsupportedLink => "Only pobb.in and pastebin.com links are supported",
            Self::Offline => "Couldn't reach the site (offline or timed out)",
            Self::NotFound => "That link doesn't exist (anymore)",
            Self::BadResponse => "The site returned something unexpected",
            Self::TooLarge => "The download is too large",
            Self::Decode(_) => "That isn't a valid Path of Building code",
        })
    }
}

fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A link becomes the raw-text URL of a known host; anything else is treated as the code itself.
pub fn classify(input: &str) -> Result<Source, ImportError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ImportError::Empty);
    }
    let Some(rest) = input.strip_prefix("https://").or_else(|| input.strip_prefix("http://")) else {
        return Ok(Source::Code(input.to_string()));
    };
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let mut parts = rest.trim_end_matches('/').split('/');
    let host = parts.next().unwrap_or("").trim_start_matches("www.");
    let segs: Vec<&str> = parts.collect();
    match (host, segs.as_slice()) {
        ("pobb.in", [id]) | ("pobb.in", [id, "raw"]) if valid_id(id) => Ok(Source::Url(format!("https://pobb.in/{id}/raw"))),
        ("pastebin.com", [id]) | ("pastebin.com", ["raw", id]) if valid_id(id) => {
            Ok(Source::Url(format!("https://pastebin.com/raw/{id}")))
        }
        _ => Err(ImportError::UnsupportedLink),
    }
}

/// Plain GET with a hard size cap. Redirects are not followed: the known hosts serve raw text directly.
fn http_get(url: &str) -> Result<String, ImportError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("PoeMate/", env!("CARGO_PKG_VERSION"), " (PoB import)"))
        .build()
        .map_err(|_| ImportError::Offline)?;
    let resp = client.get(url).send().map_err(|_| ImportError::Offline)?;
    match resp.status().as_u16() {
        200 => {}
        404 | 410 => return Err(ImportError::NotFound),
        _ => return Err(ImportError::BadResponse),
    }
    let mut body = Vec::new();
    resp.take(MAX_DOWNLOAD + 1).read_to_end(&mut body).map_err(|_| ImportError::Offline)?;
    if body.len() as u64 > MAX_DOWNLOAD {
        return Err(ImportError::TooLarge);
    }
    String::from_utf8(body).map_err(|_| ImportError::BadResponse)
}

/// Code or link in, the code (or pasted XML) itself out. This is what gets stored, so a link is only fetched once.
pub fn resolve(input: &str) -> Result<String, ImportError> {
    match classify(input)? {
        Source::Code(c) => Ok(c),
        Source::Url(u) => http_get(&u),
    }
}

/// Code, link or pasted XML in, PoB XML out (the app resolves and parses in two steps; tests use this one).
#[cfg(test)]
pub fn import(input: &str) -> Result<String, ImportError> {
    from_text(&resolve(input)?)
}

pub fn from_text(text: &str) -> Result<String, ImportError> {
    if text.trim_start().starts_with('<') {
        return Ok(text.to_string()); // already XML (a saved PoB file)
    }
    pob::decode(text).map_err(ImportError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    /// One-shot local HTTP server returning a canned response.
    fn serve(response: Vec<u8>) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let _ = s.write_all(&response);
            }
        });
        format!("http://{addr}/x")
    }

    fn ok(body: &[u8]) -> Vec<u8> {
        [format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes(), body].concat()
    }

    #[test]
    fn links_map_to_raw_urls_and_nothing_else() {
        let pobb = Source::Url("https://pobb.in/AbC-12_x/raw".into());
        assert_eq!(classify("https://pobb.in/AbC-12_x").unwrap(), pobb);
        assert_eq!(classify("  https://www.pobb.in/AbC-12_x/raw?x=1#y ").unwrap(), pobb);
        assert_eq!(classify("https://pastebin.com/Zk9").unwrap(), Source::Url("https://pastebin.com/raw/Zk9".into()));
        assert_eq!(classify("https://pastebin.com/raw/Zk9/").unwrap(), Source::Url("https://pastebin.com/raw/Zk9".into()));
        for bad in ["https://evil.example/abc", "https://pobb.in/", "https://pobb.in/a/b/c", "https://pobb.in/a b", "https://pobb.in.evil.com/abc", "https://pastebin.com/u/someone"] {
            assert_eq!(classify(bad), Err(ImportError::UnsupportedLink), "{bad}");
        }
        assert_eq!(classify("eNrtvel2"), Ok(Source::Code("eNrtvel2".into())));
        assert_eq!(classify("  "), Err(ImportError::Empty));
    }

    #[test]
    fn download_success_and_failures() {
        assert_eq!(http_get(&serve(ok(b"hello"))).unwrap(), "hello");
        assert_eq!(http_get(&serve(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec())), Err(ImportError::NotFound));
        assert_eq!(http_get(&serve(b"HTTP/1.1 500 Oops\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec())), Err(ImportError::BadResponse));
        assert_eq!(http_get(&serve(b"HTTP/1.1 302 Found\r\nLocation: http://elsewhere/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec())), Err(ImportError::BadResponse));
        assert_eq!(http_get(&serve(ok(&vec![b'a'; (MAX_DOWNLOAD + 5) as usize]))), Err(ImportError::TooLarge));
    }

    #[test]
    fn offline_is_a_friendly_error() {
        let dead = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap(); // port freed again at once
        assert_eq!(http_get(&format!("http://{dead}/")), Err(ImportError::Offline));
    }

    #[test]
    fn import_accepts_code_or_xml_and_rejects_garbage() {
        use flate2::{write::ZlibEncoder, Compression};
        use base64::Engine;
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"<PathOfBuilding/>").unwrap();
        let code = base64::engine::general_purpose::STANDARD.encode(e.finish().unwrap()).replace('+', "-").replace('/', "_");
        assert_eq!(import(&code).unwrap(), "<PathOfBuilding/>");
        assert_eq!(import("<PathOfBuilding/>").unwrap(), "<PathOfBuilding/>");
        assert!(matches!(import("garbage!!"), Err(ImportError::Decode(_))));
        assert_eq!(import(""), Err(ImportError::Empty));
    }

    #[test]
    fn imports_a_real_pobb_in_export() {
        // downloaded from https://pobb.in/Ig1GGqCA_Zp3/raw
        let xml = import(include_str!("../../fixtures/pob/real1.txt")).unwrap();
        assert!(xml.contains("<PathOfBuilding>"));
        for tag in ["<Build ", "<Tree", "<Spec ", "<Skills", "<Items"] {
            assert!(xml.contains(tag), "missing {tag}");
        }
    }

    #[test]
    #[ignore = "needs internet"]
    fn live_pobb_in_link_matches_the_fixture() {
        let live = import("https://pobb.in/Ig1GGqCA_Zp3").unwrap();
        assert_eq!(live, import(include_str!("../../fixtures/pob/real1.txt")).unwrap());
    }
}
