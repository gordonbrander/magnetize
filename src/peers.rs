use crate::url::Url;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

pub fn read_urls_from_lines<R: Read>(reader: R) -> Vec<Result<Url, UrlLinesError>> {
    let buf_reader = BufReader::new(reader);
    let mut results: Vec<Result<Url, UrlLinesError>> = Vec::new();
    for line in buf_reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                results.push(Err(UrlLinesError::from(err)));
                continue;
            }
        };
        let url = match Url::parse(&line) {
            Ok(url) => Ok(url),
            Err(err) => Err(UrlLinesError::from(err)),
        };
        results.push(url);
    }
    results
}

/// Read line-delimited URLs from a file
pub fn read_valid_urls_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Url>, io::Error> {
    let file = File::open(path)?;
    let urls = read_urls_from_lines(file)
        .into_iter()
        .filter_map(|res| match res {
            Ok(url) => Some(url),
            Err(err) => {
                eprintln!("Error reading notify URL: {}", err);
                None
            }
        })
        .collect();
    Ok(urls)
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
pub fn write_urls_to_lines<W: Write>(peers: &[Url], writer: &mut W) -> Result<(), io::Error> {
    for peer in peers {
        writeln!(writer, "{}", peer)?;
    }
    Ok(())
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
            .into_iter()
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
            .into_iter()
            .filter_map(|url| url.ok())
            .collect();

        assert_eq!(peers.len(), 0);
    }

    #[test]
    fn test_write_urls_to_lines() {
        let mut output = Vec::new();
        let peers = vec![
            Url::parse("http://example.com").unwrap(),
            Url::parse("https://test.org").unwrap(),
        ];

        write_urls_to_lines(&peers, &mut output).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "http://example.com/\nhttps://test.org/\n");
    }
}
