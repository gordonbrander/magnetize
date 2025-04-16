use std::collections::HashSet;

pub use url::{Origin, ParseError, Url};

/// Returns a set of unique origins from a list of URLs.
pub fn unique_origins(urls: &[Url]) -> HashSet<Origin> {
    HashSet::from_iter(urls.iter().map(|url| url.origin()))
}
