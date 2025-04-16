use data_encoding;
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::result;

const CID_VERSION: u8 = 0x01;
const MULTICODEC_RAW: u8 = 0x55;
const MULTIHASH_SHA256: u8 = 0x12;

/// Represents a CIDv1 with SHA-256 hash using raw codec (0x55)
/// The struct itself holds only the SHA-256 hash bytes.
/// To get a CIDV1 bytes representation, use the `to_cid_bytes` method.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Cid([u8; 32]);

impl Cid {
    /// Parse a CIDv1 from bytes representing a CIDv1
    pub fn parse_bytes(cid_bytes: Vec<u8>) -> result::Result<Self, CidError> {
        if cid_bytes.len() != 36 {
            return Err(CidError::new("Invalid CID length"));
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
            return Err(CidError::new("Invalid CID format"));
        }

        // Create a fixed-size array from the hash slice
        let mut hash = [0u8; 32];
        // Copy bytes 4 to 36 to fill the hash array
        hash.copy_from_slice(&cid_bytes[4..36]);
        Ok(Self(hash))
    }

    /// Parse a CIDv1 from a string representation.
    /// CID must be multicodec base32 lowercase encoded.
    pub fn parse(cid_str: &str) -> result::Result<Self, CidError> {
        // Check if the CID starts with "b" (multicodec code for base32 lowercase encoded)
        if !cid_str.starts_with("b") {
            return Err(CidError::new(
                "Invalid CID. CID must be multicodec base32 lowercase encoded (starts with 'b')",
            ));
        }
        let cid_body = &cid_str[1..]; // drop the "b"
        let cid_bytes = data_encoding::BASE32_NOPAD_NOCASE.decode(cid_body.as_bytes())?;
        Self::parse_bytes(cid_bytes)
    }

    /// Create a CIDv1 by hashing raw bytes
    pub fn of(bytes: impl AsRef<[u8]>) -> Self {
        let sha256_hash = Sha256::digest(bytes.as_ref());
        let sha256_hash_array: [u8; 32] = sha256_hash
            .as_slice()
            .try_into()
            .expect("SHA256 hash should be 32 bytes");
        Self(sha256_hash_array)
    }

    /// Create a CIDv1 by streaming-reading and streaming-hashing bytes from a reader
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut hasher = Sha256::new();
        // Streaming hash the bytes from the reader.
        // (Sha256 supports the Write trait)
        io::copy(reader, &mut hasher)?;
        let digest = hasher.finalize();
        let hash_array: [u8; 32] = digest
            .as_slice()
            .try_into()
            .expect("SHA256 hash should be 32 bytes");
        Ok(Self(hash_array))
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

impl std::fmt::Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug)]
pub struct CidError {
    msg: String,
}

impl CidError {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        CidError { msg: msg.into() }
    }
}

impl std::fmt::Display for CidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for CidError {}

impl From<data_encoding::DecodeError> for CidError {
    fn from(err: data_encoding::DecodeError) -> Self {
        CidError::new(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_cid_to_bytes_constructs_a_valid_cid() {
        let bytes = b"test data";
        let cid = Cid::of(bytes).to_bytes();

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
        let cid = Cid::of(text.as_bytes());

        // Known value test - this hash is for "hello world"
        let expected_cid = "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e";
        assert_eq!(cid.to_string(), expected_cid);
    }

    #[test]
    fn test_different_inputs_yield_different_cids() {
        let cid1 = Cid::of("data1".as_bytes());
        let cid2 = Cid::of("data2".as_bytes());

        // Check that different inputs create different CIDs
        assert_ne!(cid1.0, cid2.0);
        assert_ne!(cid1.to_string(), cid2.to_string());
    }

    #[test]
    fn test_identical_inputs_yield_same_cids() {
        let cid1 = Cid::of("same data".as_bytes());
        let cid2 = Cid::of("same data".as_bytes());

        // Check that identical inputs create the same CID
        assert_eq!(cid1.0, cid2.0);
        assert_eq!(cid1.to_string(), cid2.to_string());
    }

    #[test]
    fn test_cid_read_from_reader() {
        let data = b"test data";
        let mut reader = Cursor::new(data);

        let cid = Cid::read(&mut reader).unwrap().to_bytes();

        // Verify the structure is correct
        assert_eq!(cid[0], CID_VERSION);
        assert_eq!(cid[1], MULTICODEC_RAW);
        assert_eq!(cid[2], MULTIHASH_SHA256);
        assert_eq!(cid[3], 32);
        assert_eq!(cid.len(), 36);
    }
}
