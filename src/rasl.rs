use crate::cid::{self, Cid};
use crate::magnet::MagnetLink;
use crate::url::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RaslLink {
    /// CID for the data
    pub cid: Cid,
    /// RASL seed - URLs that support HTTP GET at the RASL well-known endpoint.
    /// See <https://dasl.ing/rasl.html>
    pub rs: Vec<Url>,
}

impl RaslLink {
    pub fn new(cid: Cid) -> Self {
        Self { cid, rs: vec![] }
    }

    pub fn parse(url_str: &str) -> Result<Self, Error> {
        let url = Url::parse(url_str)?;
        let authority = url.authority();
        let Some((cid_str, origins_str)) = authority.split_once(";") else {
            return Err(Error::Value(format!(
                "No authority for RASL URL: {}",
                url_str
            )));
        };
        let rs: Vec<Url> = origins_str
            .split(",")
            .filter_map(|s| Url::parse(s).ok())
            .collect();
        let cid = Cid::parse(cid_str)?;
        Ok(Self { cid, rs })
    }
}

impl From<RaslLink> for MagnetLink {
    fn from(rasl: RaslLink) -> Self {
        let mut magnet = MagnetLink::new(rasl.cid);
        magnet.rs = rasl.rs;
        magnet
    }
}

impl From<&RaslLink> for Url {
    fn from(rasl: &RaslLink) -> Self {
        let cid = rasl.cid;
        let origins = rasl
            .rs
            .iter()
            .map(|url| url.authority())
            .collect::<Vec<&str>>()
            .join(",");
        let rasl_url_string = format!("web+rasl://{cid};{origins}/");
        Url::parse(&rasl_url_string).expect("Should be able to parse url")
    }
}

impl ToString for RaslLink {
    fn to_string(&self) -> String {
        Url::from(self).to_string()
    }
}

impl From<&RaslLink> for String {
    fn from(value: &RaslLink) -> Self {
        Url::from(value).to_string()
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Value error: {0}")]
    Value(String),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("CID error: {0}")]
    Cid(#[from] cid::CidError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasl_link_parse() {
        let url = "web+rasl://bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4;example.com,test.org/";
        let rasl_link = RaslLink::parse(url).unwrap();

        assert_eq!(
            rasl_link.cid,
            Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap()
        );

        assert_eq!(rasl_link.rs.len(), 2);
        assert_eq!(rasl_link.rs[0], Url::parse("example.com").unwrap());
        assert_eq!(rasl_link.rs[1], Url::parse("test.org").unwrap());
    }

    #[test]
    fn test_rasl_link_to_string() {
        let cid =
            Cid::parse("bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4").unwrap();
        let rs = vec![
            Url::parse("https://example.com").unwrap(),
            Url::parse("https://user@test.org/extra/junk").unwrap(),
        ];

        let rasl_link = RaslLink { cid, rs };

        let string: String = (&rasl_link).into();

        assert_eq!(
            string,
            "web+rasl://bafkreiayssqzzbn2cu5mx52dvrheh7aajsermbfsn6ggtypih2rk7r6er4%3Bexample.com,user@test.org/"
        );
    }
}
