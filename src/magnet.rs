use crate::cid::{self, Cid};
use crate::util::group;
use reqwest;
use std::result;
use url::{self, Url};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MagnetLink {
    /// CID for the data
    pub cid: Cid,
    /// URLs that support HTTP GET'ing content by CID.
    /// E.g. if cdn is `https://example.com`, you can GET `https://example.com/<cid>`.
    pub cdn: Vec<String>,
    /// Web Seed (HTTP URL for the data)
    pub ws: Vec<String>,
    /// BitTorrent infohash
    pub xt: Option<String>,
    /// Display Name (file name hint)
    pub dn: Option<String>,
}

impl MagnetLink {
    /// Parse a magnet link str into a Magnet struct.
    pub fn parse(url_str: &str) -> result::Result<Self, MagnetLinkError> {
        let url = Url::parse(url_str)?;

        let query = group(
            url.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );

        let cid_string = query
            .get("cid")
            .ok_or(MagnetLinkError::MissingCid)?
            .first()
            .ok_or(MagnetLinkError::MissingCid)?;

        let cid = Cid::parse(cid_string)?;

        let cdn = query.get("cdn").map(|s| s.to_owned()).unwrap_or(Vec::new());

        let ws = query.get("ws").map(|v| v.to_owned()).unwrap_or(Vec::new());

        let xt = query
            .get("xt")
            .and_then(|xt| xt.first())
            .map(|xt| xt.to_owned());

        let dn = query
            .get("dn")
            .and_then(|dn| dn.first())
            .map(|dn| dn.to_owned());

        Ok(MagnetLink {
            cid,
            cdn,
            ws,
            xt,
            dn,
        })
    }

    /// Returns a vec of all the URLS that you can hit to download the file.
    pub fn urls(&self) -> Vec<String> {
        let cid_string = self.cid.to_string();
        let mut urls = Vec::new();

        for url in self.cdn.iter() {
            urls.push(format!("{}/{}", url.to_owned(), cid_string));
        }

        for url in self.ws.iter() {
            urls.push(url.to_owned());
        }

        urls
    }

    /// Converts the magnet link to URL string representation.
    pub fn to_string(&self) -> String {
        let mut url = Url::parse("magnet:?").unwrap();

        {
            let mut query = url.query_pairs_mut();

            query.append_pair("cid", &self.cid.to_string());

            if let Some(xt) = &self.xt {
                query.append_pair("xt", xt);
            }

            if let Some(dn) = &self.dn {
                query.append_pair("dn", dn);
            }

            for value in self.cdn.iter() {
                query.append_pair("cdn", value);
            }

            for value in self.ws.iter() {
                query.append_pair("ws", value);
            }
        }

        url.to_string()
    }
}

#[derive(Debug)]
pub enum MagnetLinkError {
    UrlParseError(url::ParseError),
    CidError(cid::CidError),
    MissingCid,
}

impl std::fmt::Display for MagnetLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MagnetLinkError::UrlParseError(err) => write!(f, "URL parse error: {}", err),
            MagnetLinkError::CidError(err) => write!(f, "CID error: {}", err),
            MagnetLinkError::MissingCid => write!(f, "Missing CID parameter"),
        }
    }
}

impl std::error::Error for MagnetLinkError {}

impl From<url::ParseError> for MagnetLinkError {
    fn from(err: url::ParseError) -> Self {
        MagnetLinkError::UrlParseError(err)
    }
}

impl From<cid::CidError> for MagnetLinkError {
    fn from(err: cid::CidError) -> Self {
        MagnetLinkError::CidError(err)
    }
}

/// Get data via magnet link, blocking the current thread until the data is retrieved.
pub fn get_blocking(mag: &MagnetLink) -> Option<Vec<u8>> {
    for url in mag.urls() {
        match reqwest::blocking::get(url) {
            Ok(response) => {
                let body = match response.bytes() {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        continue;
                    }
                };
                let cid = Cid::of(&body);

                // Check data integrity via cid
                if mag.cid != cid {
                    continue;
                }

                // Return the first successful response
                return Some(body.to_vec());
            }
            Err(_) => {
                continue;
            }
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_magnet_link() {
        let magnet_link = "magnet:?cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&ws=https://example.com/file.txt&xt=urn:btih:d41d8cd98f00b204e9800998ecf8427e&dn=example_file";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(
            result.cid.to_string(),
            "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        );
        assert_eq!(result.ws, vec!["https://example.com/file.txt"]);
        assert_eq!(
            result.xt,
            Some("urn:btih:d41d8cd98f00b204e9800998ecf8427e".to_string())
        );
        assert_eq!(result.dn, Some("example_file".to_string()));
    }

    #[test]
    fn test_parse_minimal_magnet_link() {
        let magnet_link = "magnet:?cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(
            result.cid.to_string(),
            "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        );
        assert!(result.ws.is_empty());
        assert_eq!(result.xt, None);
        assert_eq!(result.dn, None);
    }

    #[test]
    fn test_parse_multiple_ws() {
        let magnet_link = "magnet:?cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&ws=https://example1.com/file.txt&ws=https://example2.com/file.txt";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(result.ws.len(), 2);
        assert_eq!(result.ws[0], "https://example1.com/file.txt");
        assert_eq!(result.ws[1], "https://example2.com/file.txt");
    }

    #[test]
    fn test_parse_missing_cid() {
        let magnet_link = "magnet:?ws=https://example.com/file.txt";
        let result = MagnetLink::parse(magnet_link);

        assert!(matches!(result, Err(MagnetLinkError::MissingCid)));
    }

    #[test]
    fn test_parse_invalid_url() {
        let invalid_url = "not-a-magnet-link";
        let result = MagnetLink::parse(invalid_url);

        assert!(matches!(result, Err(MagnetLinkError::UrlParseError(_))));
    }

    #[test]
    fn test_to_string() {
        let magnet_link = MagnetLink {
            cid: Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap(),
            cdn: Vec::new(),
            ws: vec!["https://example.com/file.txt".to_string()],
            xt: Some("urn:btih:d41d8cd98f00b204e9800998ecf8427e".to_string()),
            dn: Some("example_file".to_string()),
        };

        let url_string = magnet_link.to_string();
        assert!(url_string.starts_with("magnet:?"), "Starts with magnet:?");

        // Parse back to verify roundtrip conversion
        let parsed = MagnetLink::parse(&url_string).unwrap();
        assert_eq!(parsed, magnet_link);
    }

    #[test]
    fn test_to_string_minimal() {
        let magnet_link = MagnetLink {
            cid: Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap(),
            cdn: Vec::new(),
            ws: vec![],
            xt: None,
            dn: None,
        };

        let url_string = magnet_link.to_string();

        // Should contain cid but empty xt and dn
        assert!(
            url_string.contains("cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4")
        );
        assert!(url_string.contains("xt="));
        assert!(url_string.contains("dn="));

        // Parse back to verify roundtrip conversion (although the empty fields will be None)
        let parsed = MagnetLink::parse(&url_string).unwrap();
        assert_eq!(parsed.cid, magnet_link.cid);
        assert!(parsed.ws.is_empty());
    }
}
