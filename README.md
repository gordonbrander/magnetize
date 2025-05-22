# magnetize

> "We have IPFS at home"

Content-addressed data over HTTP using magnet links.

```url
magnet:?xt=urn:cid:<CID>&ws=<URL>&rs=<URL>
```

Minimum viable decentralization is n > 1. Magnetize achieves minimum viable decentralization by combining a CID with multiple redundant places to GET it over HTTP.

Magnetized links are [self-certifying](https://jaygraber.medium.com/web3-is-self-certifying-9dad77fd8d81). You do not have to trust the servers in the magnet link to serve the correct data. The CID is a cryptographic proof for the data, ensuring that the data is what it says it is.

Magnetize offers a CLI with several tools for content-addressed data over HTTP:

- `mag get <MAGNET_URL>`: fetch content addressed data over HTTP(S) using a magnet link. This command will try locations until it finds one that succeeds.
- `mag link <URL>...`: create a magnet link from one or more HTTP(s) URLs.
- `mag serve <DIR>`: simple file server for content addressed data. The server is written in Rust, so is reasonably fast.
- `mag add <FILE>`: add content addressed data from a file. This command will create a new file in the working directory who's name is the CID and who's contents is the file bytes.

See `mag --help` for a full list of commands and features.

## Magnet links

Magnet links are used for locating data on BitTorrent. However, they are also a general-purpose protocol for bundling together multiple ways to fetch the same data. Magnetize extends magnet links, adding parameters to support content-addressed data over HTTP.

Magnet link parameters supported by Magnetize:

- `xt=urn:cid:<CID>`: The CID.
- `xt=urn:btmh:<INFOHASH>`:A BitTorrent infohash, allowing this magnet link to be used with BitTorrent clients.
- `ws=<URL>`: "Web Seed". A direct HTTP link to the data matching the CID/infohash payload.
- `rs=<URL>`: URL pointing to a CDN that supports HTTP GET for CIDs at the [well-known RASL endpoint](https://dasl.ing/rasl.html).
- `dn=<FILE>`: "Display Name". A suggested file name.

Magnetize [aims to be compatible with common magnet parameters](https://wiki.theory.org/BitTorrent_Magnet-URI_Webseeding). This means you can construct hybrid magnet links which work with both Magnetize and [BitTorrent](https://blog.libtorrent.org/2020/09/bittorrent-v2/). Just include the `xt` parameter:

```url
magnet:?xt=urn:btmh:<INFOHASH>&urn:cid:<CID>&rs=https://example.com
```

When used with BitTorrent, you can think of the `ws` and `rs` parameters as high availability peers to try first, while falling back to BitTorrent's DHT when an HTTP source is unavailable.

## CIDs

Magnetize supports one kind of IPFS CID:

- Multibase: base32
- CID: v1
- Multicodec: raw bytes
- Multihash: sha256

In string form, the cid is always encoded in multibase lowercase base32. This means the CID string will always have prefix of `b` (multibase flag for base32).

Once deserialized to bytes, a CIDv1 has the following byte structure:

```
<version><multicodec><multihash><size><digest>
```

1. A CID version number, which is currently always 1.
2. A content codec, which is currently always `0x55` (multicodec flag for raw bytes)
3. A hash function, which is currently always `0x12` (multihash flag for sha256)
4. A hash size, which is the size in bytes of the hash digest. Always `32` for sha256.
5. A hash digest, which is the hash of the raw bytes.

This CID type is described in more detail here: [dasl.ing/cid.html](https://dasl.ing/cid.html).

## Development

### Installing binaries on your path with Cargo

From the project directory:

```bash
cargo install --path .
```

This will install the binaries to `~/.cargo/bin`, which is usually added to your path by the Rust installer.
