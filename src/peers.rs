use crate::url::Url;
use std::io::{self, BufRead, BufReader, Read, Write};

/// Read line-delimited URLs from a reader, such as an open file.
pub fn read_peers<R: Read>(reader: R) -> Result<Vec<Url>, PeersError> {
    let buf_reader = BufReader::new(reader);
    let mut peers = Vec::new();
    for line in buf_reader.lines() {
        let url = Url::parse(&line?)?;
        peers.push(url);
    }
    Ok(peers)
}

/// Write line-delimited peers to a writer, such as an open file.
pub fn write_peers<W: Write>(peers: &[Url], writer: &mut W) -> Result<(), PeersError> {
    for peer in peers {
        writeln!(writer, "{}", peer)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum PeersError {
    IoError(io::Error),
    UrlError(url::ParseError),
}

impl std::fmt::Display for PeersError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PeersError::IoError(err) => write!(f, "IO error: {}", err),
            PeersError::UrlError(err) => write!(f, "URL parse error: {}", err),
        }
    }
}

impl std::error::Error for PeersError {}

impl From<io::Error> for PeersError {
    fn from(err: io::Error) -> Self {
        PeersError::IoError(err)
    }
}

impl From<url::ParseError> for PeersError {
    fn from(err: url::ParseError) -> Self {
        PeersError::UrlError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_peers() {
        let input = "http://example.com\nhttps://test.org\n";
        let reader = Cursor::new(input);

        let peers = read_peers(reader).unwrap();

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].as_str(), "http://example.com/");
        assert_eq!(peers[1].as_str(), "https://test.org/");
    }

    #[test]
    fn test_read_peers_invalid_url() {
        let input = "not a valid url";
        let reader = Cursor::new(input);

        let result = read_peers(reader);

        assert!(result.is_err());
        if let Err(PeersError::UrlError(_)) = result {
            // Expected error
        } else {
            panic!("Expected UrlError, got {:?}", result);
        }
    }

    #[test]
    fn test_write_peers() {
        let mut output = Vec::new();
        let peers = vec![
            Url::parse("http://example.com").unwrap(),
            Url::parse("https://test.org").unwrap(),
        ];

        write_peers(&peers, &mut output).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "http://example.com/\nhttps://test.org/\n");
    }
}
