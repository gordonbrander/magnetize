use crate::cid::Cid;
use crate::error::Error;
use std::collections::HashMap;
use std::result;
use url::{self, Url};

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

fn index_query(pairs: Vec<(String, String)>) -> HashMap<String, Vec<String>> {
    let mut query: HashMap<String, Vec<String>> = HashMap::new();

    pairs.into_iter().for_each(|(key, value)| {
        query.entry(key).or_insert(Vec::new()).push(value);
    });

    query
}

impl MagnetLink {
    /// Parse a magnet link str into a Magnet struct.
    pub fn parse(url_str: &str) -> result::Result<Self, MagnetLinkError> {
        let url = Url::parse(url_str)?;

        let query = index_query(
            url.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );

        let cid_string = query
            .get("cid")
            .ok_or(MagnetLinkError::MissingCid)?
            .first()
            .ok_or(MagnetLinkError::MissingCid)?;

        let cid = Cid::from(cid_string.as_str());

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
}

#[derive(Debug)]
pub enum MagnetLinkError {
    UrlParseError(url::ParseError),
    MissingCid,
}

impl std::fmt::Display for MagnetLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MagnetLinkError::MissingCid => write!(f, "Missing CID parameter"),
            MagnetLinkError::UrlParseError(err) => write!(f, "URL parse error: {}", err),
        }
    }
}

impl From<url::ParseError> for MagnetLinkError {
    fn from(err: url::ParseError) -> Self {
        MagnetLinkError::UrlParseError(err)
    }
}

impl From<MagnetLinkError> for Error {
    fn from(err: MagnetLinkError) -> Self {
        Error::MagnetLinkError(err)
    }
}
