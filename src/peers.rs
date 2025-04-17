use crate::url::Url;
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

/// Read line-delimited URLs from a reader.
/// Returns an iterator of `Result<Url, UrlLinesError>`.
pub fn read_urls_from_lines<R: Read>(
    reader: R,
) -> impl Iterator<Item = Result<Url, UrlLinesError>> {
    let buf_reader = BufReader::new(reader);
    buf_reader.lines().map(|line| {
        let line = match line {
            Ok(line) => line,
            Err(err) => return Err(UrlLinesError::from(err)),
        };
        match Url::parse(&line) {
            Ok(url) => Ok(url),
            Err(err) => Err(UrlLinesError::from(err)),
        }
    })
}

/// Read line-delimited URLs from a reader
/// Discards invalid URLs
pub fn read_valid_urls_from_lines<R: Read>(reader: R) -> impl Iterator<Item = url::Url> {
    read_urls_from_lines(reader).filter_map(|res| res.ok())
}

/// Read line-delimited URLs from a file path
/// Discards invalid URLs
pub fn read_valid_urls_from_path(path: &Path) -> Result<Vec<url::Url>, UrlLinesError> {
    let file = std::fs::File::open(path)?;
    Ok(read_valid_urls_from_lines(file).collect())
}

#[derive(Debug)]
pub enum UrlLinesError {
    IoError(io::Error),
    UrlError(url::ParseError),
}

impl std::fmt::Display for UrlLinesError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UrlLinesError::IoError(err) => write!(f, "IO error: {}", err),
            UrlLinesError::UrlError(err) => write!(f, "URL parse error: {}", err),
        }
    }
}

impl std::error::Error for UrlLinesError {}

impl From<io::Error> for UrlLinesError {
    fn from(err: io::Error) -> Self {
        UrlLinesError::IoError(err)
    }
}

impl From<url::ParseError> for UrlLinesError {
    fn from(err: url::ParseError) -> Self {
        UrlLinesError::UrlError(err)
    }
}

/// Write line-delimited peers to a writer, such as an open file.
pub fn write_urls_to_lines<W: Write>(writer: &mut W, peers: &[Url]) -> Result<(), io::Error> {
    for peer in peers {
        writeln!(writer, "{}", peer)?;
    }
    Ok(())
}

/// Should we listen to notifications about this peer?
/// - Deny list is always honored
/// - Otherwise, notifications are restricted to allow list unless allow_all is true
pub fn should_allow_peer(
    peer: &Url,
    allow: &HashSet<url::Origin>,
    deny: &HashSet<url::Origin>,
    allow_all: bool,
) -> bool {
    let peer_origin = peer.origin();
    // Always honor deny list
    if deny.contains(&peer_origin) {
        return false;
    }
    // If peer is not in the deny list, and we allow all, return true
    if allow_all {
        return true;
    }
    // Otherwise check against allow list
    allow.contains(&peer_origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_urls_from_lines() {
        let input = "http://example.com\nhttps://test.org\n";
        let reader = Cursor::new(input);

        let peers: Vec<Url> = read_urls_from_lines(reader)
            .filter_map(|url| url.ok())
            .collect();

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].as_str(), "http://example.com/");
        assert_eq!(peers[1].as_str(), "https://test.org/");
    }

    #[test]
    fn test_read_urls_from_lines_invalid_url() {
        let input = "not a valid url";
        let reader = Cursor::new(input);

        let peers: Vec<Url> = read_urls_from_lines(reader)
            .filter_map(|url| url.ok())
            .collect();

        assert_eq!(peers.len(), 0);
    }

    #[test]
    fn test_read_valid_urls_from_lines() {
        let input = "http://example.com\ninvalid url\nhttps://test.org\n";
        let reader = Cursor::new(input);

        let peers: Vec<Url> = read_valid_urls_from_lines(reader).collect();

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].as_str(), "http://example.com/");
        assert_eq!(peers[1].as_str(), "https://test.org/");
    }

    #[test]
    fn test_write_urls_to_lines() {
        let mut output = Vec::new();
        let peers = vec![
            Url::parse("http://example.com").unwrap(),
            Url::parse("https://test.org").unwrap(),
        ];

        write_urls_to_lines(&mut output, &peers).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "http://example.com/\nhttps://test.org/\n");
    }

    #[test]
    fn test_should_allow_peer() {
        let peer = Url::parse("https://example.com/resource").unwrap();
        let peer2 = Url::parse("https://allowed.com/resource").unwrap();
        let peer3 = Url::parse("https://denied.com/resource").unwrap();

        let allow = HashSet::from_iter(vec![
            url::Url::parse("https://allowed.com").unwrap().origin(),
        ]);

        let deny = HashSet::from_iter(vec![
            url::Url::parse("https://denied.com").unwrap().origin(),
        ]);

        // Test with allow_all = true
        assert!(
            should_allow_peer(&peer, &allow, &deny, true),
            "When allow_all is true, non-denied peer should be notified"
        );

        assert!(
            should_allow_peer(&peer2, &allow, &deny, true),
            "When allow_all is true, allowed peer should be notified"
        );

        assert!(
            !should_allow_peer(&peer3, &allow, &deny, true),
            "When allow_all is true, denied peer should not be notified"
        );

        // Test with allow_all = false
        assert!(
            !should_allow_peer(&peer, &allow, &deny, false),
            "When allow_all is false, non-allowed peer should not be notified"
        );

        assert!(
            should_allow_peer(&peer2, &allow, &deny, false),
            "When allow_all is false, allowed peer should be notified"
        );

        assert!(
            !should_allow_peer(&peer3, &allow, &deny, false),
            "When allow_all is false, denied peer should not be notified"
        );
    }
}
