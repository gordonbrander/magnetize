use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cid::{self, Cid};
use crate::url::{Url, into_btmh_urn_str, parse_btmh_urn_str, parse_cid_urn_str};
use crate::util::group;
use std::result;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MagnetLink {
    /// CID for the data
    pub cid: Cid,
    /// RASL seed - URLs that support HTTP GET at the RASL well-known endpoint.
    /// See <https://dasl.ing/rasl.html>
    pub rs: Vec<Url>,
    /// Web Seed (HTTP URL for the data)
    pub ws: Vec<Url>,
    /// BitTorrent infohash
    pub btmh: Option<String>,
    /// Display Name (file name hint)
    pub dn: Option<String>,
}

impl MagnetLink {
    /// Create a new MagnetLink with only a CID.
    pub fn new(cid: Cid) -> Self {
        Self {
            cid,
            rs: Vec::new(),
            ws: Vec::new(),
            btmh: None,
            dn: None,
        }
    }

    /// Parse a magnet link str into a Magnet struct.
    pub fn parse(url_str: &str) -> result::Result<Self, Error> {
        let url = Url::parse(url_str)?;

        let query = group(
            url.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );

        let xts = query.get("xt").ok_or(Error::InvalidMagnetLink(
            "xt parameter not found".to_string(),
        ))?;

        let cid = xts.iter().find_map(|xt| parse_cid_urn_str(xt).ok()).ok_or(
            Error::InvalidMagnetLink("cid:urn: xt parameter incorrect or missing".to_string()),
        )?;

        let btmh = xts.iter().find_map(|xt| parse_btmh_urn_str(xt).ok());

        let rs = query
            .get("rs")
            .map(|v| v.into_iter().filter_map(|s| Url::parse(s).ok()).collect())
            .unwrap_or(Vec::new());

        let ws = query
            .get("ws")
            .map(|v| v.into_iter().filter_map(|s| Url::parse(s).ok()).collect())
            .unwrap_or(Vec::new());

        let dn = query
            .get("dn")
            .and_then(|dn| dn.first())
            .map(|dn| dn.to_owned());

        Ok(MagnetLink {
            cid,
            rs,
            ws,
            btmh,
            dn,
        })
    }

    /// Returns a vec of all the URLS that you can hit to download the file.
    pub fn urls(&self) -> Vec<Url> {
        let cid_string = self.cid.to_string();
        // Join CID to the end of RASL URLs
        let rasl_urls = self
            .rs
            .iter()
            .filter_map(|url| into_rasl_url(url).ok())
            .filter_map(|rasl_url| rasl_url.join(&cid_string).ok());
        let ws_urls = self.ws.clone().into_iter();
        rasl_urls.chain(ws_urls).collect()
    }
}

impl From<&MagnetLink> for Url {
    fn from(magnet: &MagnetLink) -> Self {
        let mut url = Url::parse("magnet:?").unwrap();

        {
            let mut query = url.query_pairs_mut();

            let cid_urn =
                Url::try_from(&magnet.cid).expect("Should be able to construct URL from cid");
            query.append_pair("xt", &cid_urn.to_string());

            if let Some(btmh) = &magnet.btmh {
                query.append_pair("xt", into_btmh_urn_str(btmh).as_str());
            }

            if let Some(dn) = &magnet.dn {
                query.append_pair("dn", dn);
            }

            for value in magnet.rs.iter() {
                query.append_pair("rs", value.as_str());
            }

            for value in magnet.ws.iter() {
                query.append_pair("ws", value.as_str());
            }
        }

        url
    }
}

impl ToString for MagnetLink {
    fn to_string(&self) -> String {
        Url::from(self).to_string()
    }
}

impl From<&MagnetLink> for String {
    fn from(magnet: &MagnetLink) -> Self {
        Url::from(magnet).to_string()
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid magnet link")]
    InvalidMagnetLink(String),
    #[error("Invalid RASL endpoint")]
    InvalidRaslEndpoint(String),
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("CID error: {0}")]
    Cid(String),
}

impl From<cid::CidError> for Error {
    fn from(err: cid::CidError) -> Self {
        Error::Cid(err.to_string())
    }
}

/// Refactors a URL into a RASL CDN URL if possible.
/// We use this as a sanitization step when parsing `rs` param.
/// See <https://dasl.ing/rasl.html>.
fn into_rasl_url(url: &Url) -> Result<Url, Error> {
    let authority = url.authority();
    if authority == "" {
        return Err(Error::InvalidRaslEndpoint(format!(
            "URL has no authority: {}",
            url
        )));
    }
    let rasl_url_string = format!("https://{authority}/.well-known/rasl/");
    let rasl_url = Url::parse(&rasl_url_string)?;
    Ok(rasl_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_magnet_link() {
        let magnet_link = "magnet:?xt=urn:cid:bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&ws=https://example.com/file.txt&xt=urn:btmh:d41d8cd98f00b204e9800998ecf8427e&dn=example_file";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(
            result.cid.to_string(),
            "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        );
        assert_eq!(
            result.ws,
            vec![Url::parse("https://example.com/file.txt").unwrap()]
        );
        assert_eq!(
            result.btmh,
            Some("d41d8cd98f00b204e9800998ecf8427e".to_string())
        );
        assert_eq!(result.dn, Some("example_file".to_string()));
    }

    #[test]
    fn test_parse_minimal_magnet_link() {
        let magnet_link =
            "magnet:?xt=urn:cid:bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(
            result.cid.to_string(),
            "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        );
        assert!(result.ws.is_empty());
        assert_eq!(result.btmh, None);
        assert_eq!(result.dn, None);
    }

    #[test]
    fn test_parse_multiple_ws() {
        let magnet_link = "magnet:?xt=urn:cid:bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&ws=https://example1.com/file.txt&ws=https://example2.com/file.txt";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(result.ws.len(), 2);
        assert_eq!(
            result.ws[0],
            Url::parse("https://example1.com/file.txt").unwrap()
        );
        assert_eq!(
            result.ws[1],
            Url::parse("https://example2.com/file.txt").unwrap()
        );
    }

    #[test]
    fn test_parse_missing_cid() {
        let magnet_link = "magnet:?ws=https://example.com/file.txt";
        let result = MagnetLink::parse(magnet_link);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_url() {
        let invalid_url = "not-a-magnet-link";
        let result = MagnetLink::parse(invalid_url);

        assert!(matches!(result, Err(Error::UrlParseError(_))));
    }

    #[test]
    fn test_to_string() {
        let magnet_link = MagnetLink {
            cid: Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap(),
            rs: Vec::new(),
            ws: vec![Url::parse("https://example.com/file.txt").unwrap()],
            btmh: Some("d41d8cd98f00b204e9800998ecf8427e".to_string()),
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
            rs: Vec::new(),
            ws: vec![],
            btmh: None,
            dn: None,
        };

        let url_string = magnet_link.to_string();

        // Should contain cid but empty xt and dn
        assert!(url_string.contains(
            "?xt=urn%3Acid%3Abafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        ));
        assert!(!url_string.contains("ws="), "Does not contain ws=");
        assert!(!url_string.contains("dn="), "Does not contain dn=");

        // Parse back to verify roundtrip conversion (although the empty fields will be None)
        let parsed = MagnetLink::parse(&url_string).unwrap();
        assert_eq!(parsed.cid, magnet_link.cid);
        assert!(parsed.ws.is_empty());
    }

    #[test]
    fn test_urls_method() {
        let cid_str = "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4";
        let magnet_link = MagnetLink {
            cid: Cid::parse(cid_str).unwrap(),
            rs: vec![
                Url::parse("https://cdn1.example.com/").unwrap(),
                Url::parse("https://cdn2.example.com/junk/at-the/end").unwrap(),
            ],
            ws: vec![
                Url::parse("https://direct1.example.com/file.txt").unwrap(),
                Url::parse("https://direct2.example.com/another-file.txt").unwrap(),
            ],
            btmh: None,
            dn: None,
        };

        let urls = magnet_link.urls();

        // Check that we have the expected number of URLs
        assert_eq!(urls.len(), 4);

        // Check RASL URLs have CID appended
        assert!(
            urls.contains(
                &Url::parse(&format!(
                    "https://cdn1.example.com/.well-known/rasl/{}",
                    cid_str
                ))
                .unwrap()
            )
        );
        assert!(
            urls.contains(
                &Url::parse(&format!(
                    "https://cdn2.example.com/.well-known/rasl/{}",
                    cid_str
                ))
                .unwrap()
            )
        );

        // Check WS URLs are left as-is
        assert!(urls.contains(&Url::parse("https://direct1.example.com/file.txt").unwrap()));
        assert!(
            urls.contains(&Url::parse("https://direct2.example.com/another-file.txt").unwrap())
        );
    }
}
