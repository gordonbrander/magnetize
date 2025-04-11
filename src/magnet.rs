use crate::cid::{self, Cid};
use crate::util::group;
use std::result;
use url::{self, Url};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MagnetLink {
    /// CID for the data
    pub cid: Cid,
    /// Exact Source (HTTP URL for the data)
    pub xs: Vec<String>,
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

        let xs = query
            .get("xs")
            .map(|xs| xs.to_owned())
            .unwrap_or(Vec::new());

        let xt = query
            .get("xt")
            .and_then(|xt| xt.first())
            .map(|xt| xt.to_owned());

        let dn = query
            .get("dn")
            .and_then(|dn| dn.first())
            .map(|dn| dn.to_owned());

        Ok(MagnetLink { cid, xs, xt, dn })
    }

    /// Converts the magnet link to URL string representation.
    pub fn to_string(&self) -> String {
        let mut url = Url::parse("magnet:?").unwrap();

        {
            let mut query = url.query_pairs_mut();

            query
                .append_pair("cid", &self.cid.to_string())
                .append_pair("xt", &self.xt.as_ref().unwrap_or(&"".to_string()))
                .append_pair("dn", &self.dn.as_ref().unwrap_or(&"".to_string()));

            for value in self.xs.iter() {
                query.append_pair("xs", &value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_magnet_link() {
        let magnet_link = "magnet:?cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&xs=https://example.com/file.txt&xt=urn:btih:d41d8cd98f00b204e9800998ecf8427e&dn=example_file";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(
            result.cid.to_string(),
            "bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4"
        );
        assert_eq!(result.xs, vec!["https://example.com/file.txt"]);
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
        assert!(result.xs.is_empty());
        assert_eq!(result.xt, None);
        assert_eq!(result.dn, None);
    }

    #[test]
    fn test_parse_multiple_xs() {
        let magnet_link = "magnet:?cid=bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4&xs=https://example1.com/file.txt&xs=https://example2.com/file.txt";
        let result = MagnetLink::parse(magnet_link).unwrap();

        assert_eq!(result.xs.len(), 2);
        assert_eq!(result.xs[0], "https://example1.com/file.txt");
        assert_eq!(result.xs[1], "https://example2.com/file.txt");
    }

    #[test]
    fn test_parse_missing_cid() {
        let magnet_link = "magnet:?xs=https://example.com/file.txt";
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
            xs: vec!["https://example.com/file.txt".to_string()],
            xt: Some("urn:btih:d41d8cd98f00b204e9800998ecf8427e".to_string()),
            dn: Some("example_file".to_string()),
        };

        let url_string = magnet_link.to_string();

        // Parse back to verify roundtrip conversion
        let parsed = MagnetLink::parse(&url_string).unwrap();
        assert_eq!(parsed, magnet_link);
    }

    #[test]
    fn test_to_string_minimal() {
        let magnet_link = MagnetLink {
            cid: Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap(),
            xs: vec![],
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
        assert!(parsed.xs.is_empty());
    }
}
