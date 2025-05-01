use crate::cid::Cid;
use thiserror::Error;
pub use url::{Origin, ParseError, Url};

/// Parse `urn:cid` into Cid
pub fn parse_cid_urn_str(urn_str: &str) -> Result<Cid, Error> {
    let cid_body = urn_str
        .strip_prefix("urn:cid:")
        .ok_or(Error::Value("Not a urn:cid".to_string()))?;
    Cid::parse(cid_body).map_err(|e| Error::Value(e.to_string()))
}

impl TryFrom<&Url> for Cid {
    type Error = Error;

    fn try_from(url: &Url) -> Result<Cid, Self::Error> {
        let url_str = url.as_str();
        parse_cid_urn_str(url_str)
    }
}

impl TryFrom<&Cid> for Url {
    type Error = Error;

    fn try_from(cid: &Cid) -> Result<Url, Self::Error> {
        let cid_str = cid.to_string();
        Url::parse(&format!("urn:cid:{}", cid_str)).map_err(|e| Error::Url(e))
    }
}

/// Parse `urn:btmh` into btmh string
pub fn parse_btmh_urn_str(urn_str: &str) -> Result<String, Error> {
    let btmh = urn_str
        .strip_prefix("urn:btmh:")
        .ok_or(Error::Value("Not a urn:btmh".to_string()))?;
    Ok(btmh.to_string())
}

pub fn into_btmh_urn_str(btmh: &str) -> String {
    format!("urn:btmh:{}", btmh)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid URL")]
    Url(#[from] ParseError),
    #[error("Invalid value")]
    Value(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cid_urn_str() {
        // Valid CID URN
        let valid_urn = "urn:cid:bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e";
        let cid = parse_cid_urn_str(valid_urn).unwrap();
        assert_eq!(
            cid,
            Cid::parse("bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap()
        );

        // Not a URN
        let invalid_urn = "noturn:cid:bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e";
        let result = parse_cid_urn_str(invalid_urn);
        assert!(result.is_err());
        match result {
            Err(Error::Value(msg)) => assert!(msg.contains("Not a urn:cid")),
            _ => panic!("Expected Value error"),
        }

        // Invalid CID
        let invalid_cid = "urn:cid:invalidcid";
        let result = parse_cid_urn_str(invalid_cid);
        assert!(result.is_err());
    }
}
