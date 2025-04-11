use crate::error::{Error, Result};
use data_encoding;
use sha2::{Digest, Sha256};

const CID_VERSION: u8 = 0x01;
const MULTICODEC_RAW: u8 = 0x55;
const MULTIHASH_SHA256: u8 = 0x12;

/// Represents a CIDv1 with SHA-256 hash using raw codec (0x55)
/// The struct itself holds only the SHA-256 hash bytes.
/// To get a CIDV1 bytes representation, use the `to_cid_bytes` method.
pub struct Cid(Vec<u8>);

impl Cid {
    /// Create a CIDv1 from the bytes of a SHA-256 hash
    pub fn from_sha256(sha256_hash: Vec<u8>) -> Self {
        Self(sha256_hash)
    }

    /// Construct a CIDv1 from bytes representing a CIDv1
    pub fn from_cid_bytes(cid_bytes: Vec<u8>) -> Result<Self> {
        if cid_bytes.len() != 36 {
            return Err(Error::ValueError("Invalid CID length".to_string()));
        }

        let version = cid_bytes[0];
        let codec = cid_bytes[1];
        let hash_algo = cid_bytes[2];
        let hash_len = cid_bytes[3];

        if version != CID_VERSION
            || codec != MULTICODEC_RAW
            || hash_algo != MULTIHASH_SHA256
            || hash_len != 32
        {
            return Err(Error::ValueError("Invalid CID format".to_string()));
        }

        let hash = cid_bytes[4..].to_vec();
        Ok(Self(hash))
    }

    /// Parse a CIDv1 from a string representation.
    /// CID must be multicodec base32 lowercase encoded.
    pub fn from_cid_str(cid_str: &str) -> Result<Self> {
        // Check if the CID starts with "b" (multicodec code for base32 lowercase encoded)
        if !cid_str.starts_with("b") {
            return Err(Error::ValueError(
                "Invalid CID. CID must be multicodec base32 lowercase encoded (starts with 'b')"
                    .to_string(),
            ));
        }
        let cid_body = &cid_str[1..]; // drop the "b"
        let cid_bytes = data_encoding::BASE32_NOPAD_NOCASE.decode(cid_body.as_bytes())?;
        Self::from_cid_bytes(cid_bytes)
    }

    /// Create a CIDv1 by hashing raw bytes
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        let sha256_hash = Sha256::digest(bytes.as_ref());
        Self::from_sha256(sha256_hash.to_vec())
    }

    /// Get the byte representation of a valid CIDv1
    /// See https://dasl.ing/cid.html
    pub fn to_bytes(&self) -> Vec<u8> {
        // Initialize a vector to hold the CID bytes
        // 4 bytes for version, codec, hash algo, length + 32 for hash
        let mut cid_bytes = Vec::with_capacity(36);

        // version 1
        cid_bytes.push(CID_VERSION);

        // raw codec (0x55)
        cid_bytes.push(MULTICODEC_RAW);

        // sha2-256 hash algorithm (0x12)
        cid_bytes.push(MULTIHASH_SHA256);

        // hash length (32 bytes for sha256)
        cid_bytes.push(32);

        // append the hash itself
        cid_bytes.extend_from_slice(&self.0.as_ref());

        // Return the CID bytes
        cid_bytes
    }

    /// Get the string representation of a valid CIDv1
    /// See https://dasl.ing/cid.html
    pub fn to_string(&self) -> String {
        let cid_bytes = self.to_bytes();

        // Convert to base32
        let mut encoded = String::with_capacity(cid_bytes.len() * 2);

        // Add the "b" multibase prefix for base32
        encoded.push('b');

        // Encode the CID bytes using base32 lower
        encoded.push_str(&data_encoding::BASE32_NOPAD.encode(&cid_bytes));

        // Return the encoded string (lowercase)
        encoded.to_lowercase()
    }
}

impl From<String> for Cid {
    fn from(s: String) -> Self {
        Self::new(s.as_bytes())
    }
}

impl From<&str> for Cid {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_to_bytes_constructs_a_valid_cid() {
        let bytes = b"test data";
        let cid = Cid::new(bytes).to_bytes();

        // Verify the structure is correct
        assert_eq!(cid[0], CID_VERSION);
        assert_eq!(cid[1], MULTICODEC_RAW);
        assert_eq!(cid[2], MULTIHASH_SHA256);
        assert_eq!(cid[3], 32);
        assert_eq!(cid.len(), 36);
    }

    #[test]
    fn test_cid_to_string_constructs_a_valid_cid() {
        let text = "hello world";
        let cid = Cid::from(text);

        // Known value test - this hash is for "hello world"
        let expected_cid = "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e";
        assert_eq!(cid.to_string(), expected_cid);
    }

    #[test]
    fn test_different_inputs_yield_different_cids() {
        let cid1 = Cid::from("data1");
        let cid2 = Cid::from("data2");

        // Check that different inputs create different CIDs
        assert_ne!(cid1.0, cid2.0);
        assert_ne!(cid1.to_string(), cid2.to_string());
    }

    #[test]
    fn test_identical_inputs_yield_same_cids() {
        let cid1 = Cid::from("same data");
        let cid2 = Cid::from("same data");

        // Check that identical inputs create the same CID
        assert_eq!(cid1.0, cid2.0);
        assert_eq!(cid1.to_string(), cid2.to_string());
    }
}
