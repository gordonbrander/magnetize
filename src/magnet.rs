use crate::cid::{Cid, CidError};
use crate::url::Url;
use crate::util::group;
use std::result;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MagnetLink {
    /// CID for the data
    pub cid: Cid,
    /// URLs that support HTTP GET'ing content by CID.
    /// E.g. if cdn is `https://example.com`, you can GET `https://example.com/<cid>`.
    pub cdn: Vec<Url>,
    /// Web Seed (HTTP URL for the data)
    pub ws: Vec<Url>,
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

        let cdn = query
            .get("cdn")
            .map(|v| v.into_iter().filter_map(|s| Url::parse(s).ok()).collect())
            .unwrap_or(Vec::new());

        let ws = query
            .get("ws")
            .map(|v| v.into_iter().filter_map(|s| Url::parse(s).ok()).collect())
            .unwrap_or(Vec::new());

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
    pub fn urls(&self) -> Vec<Url> {
        let cid_string = self.cid.to_string();
        let cdn_urls = self.cdn.iter().filter_map(|url| url.join(&cid_string).ok());
        let ws_urls = self.ws.clone().into_iter();
        cdn_urls.chain(ws_urls).collect()
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
                query.append_pair("cdn", value.as_str());
            }

            for value in self.ws.iter() {
                query.append_pair("ws", value.as_str());
            }
        }

        url.to_string()
    }
}

#[derive(Debug)]
pub enum MagnetLinkError {
    UrlParseError(url::ParseError),
    CidError(CidError),
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

impl From<CidError> for MagnetLinkError {
    fn from(err: CidError) -> Self {
        MagnetLinkError::CidError(err)
    }
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
        assert_eq!(
            result.ws,
            vec![Url::parse("https://example.com/file.txt").unwrap()]
        );
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
            ws: vec![Url::parse("https://example.com/file.txt").unwrap()],
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
        assert!(!url_string.contains("ws="), "Does not contain ws=");
        assert!(!url_string.contains("dn="), "Does not contain dn=");

        // Parse back to verify roundtrip conversion (although the empty fields will be None)
        let parsed = MagnetLink::parse(&url_string).unwrap();
        assert_eq!(parsed.cid, magnet_link.cid);
        assert!(parsed.ws.is_empty());
    }
}
