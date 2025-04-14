use crate::cid::Cid;
use crate::url::Url;
use reqwest;

pub type Client = reqwest::Client;

pub fn build_client(timeout: std::time::Duration) -> Result<Client, reqwest::Error> {
    let client = reqwest::ClientBuilder::new().timeout(timeout).build()?;
    Ok(client)
}

/// HEAD CID, to check if a CID exists at a URL
/// Note that this function does not perform an integrity check, since HEAD requests do not include the body.
pub async fn head_cid(
    client: &Client,
    url: &Url,
    cid: &Cid,
) -> Result<reqwest::Response, RequestError> {
    let cid_str = cid.to_string();
    let url = url.join(&cid_str)?;
    let response = client.head(url).send().await?;
    Ok(response)
}

/// Fetches a CID from a URL, does integrity check.
/// Returns the bytes if file is found and integrity check passes.
pub async fn get_cid(client: &Client, url: &Url, cid: &Cid) -> Result<Vec<u8>, RequestError> {
    let cid_str = cid.to_string();
    let url = url.join(&cid_str)?;
    let response = client.get(url).send().await?;
    let body = response.bytes().await?;

    // Generate CID from response
    let body_cid = Cid::of(&body);

    // Do integrity check
    if !&body_cid.eq(cid) {
        return Err(RequestError::IntegrityError(format!(
            "Response doesn't match CID\
                Expected: {}\
                Got: {}",
            cid, body_cid
        )));
    }

    // Return the bytes
    Ok(body.to_vec())
}

#[derive(Debug)]
pub enum RequestError {
    RequestError(reqwest::Error),
    UrlParseError(url::ParseError),
    IntegrityError(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::RequestError(err) => write!(f, "Request Error: {}", err),
            RequestError::UrlParseError(err) => write!(f, "URL Parse Error: {}", err),
            RequestError::IntegrityError(err) => write!(f, "Integrity Error: {}", err),
        }
    }
}

impl std::error::Error for RequestError {}

impl From<reqwest::Error> for RequestError {
    fn from(err: reqwest::Error) -> Self {
        RequestError::RequestError(err)
    }
}

impl From<url::ParseError> for RequestError {
    fn from(err: url::ParseError) -> Self {
        RequestError::UrlParseError(err)
    }
}
