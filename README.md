# magnetize

> "We have IPFS at home"

Content-addressed data over HTTP using magnet links.

```url
magnet:?cid=<CID>&cdn=<URL>&cdn=<URL>
```

Minimum viable decentralization is n > 1. Magnetize achieves minimum viable decentralization by combining a CID with multiple redundant places to GET it over HTTP.

Notably, you do not have to trust the servers listed in the magnet link to serve the correct data. The CID acts as a cryptographic proof for the data, ensuring you get exactly the data you requested.

Magnetize offers a CLI with several tools for content-addressed data over HTTP:

- `mag get <MAGNET_URL>`: fetch content addressed data over HTTP(S) using a magnet link. This command will try locations until it finds one that succeeds.
- `mag link <URL>...`: create a magnet link from one or more HTTP(s) URLs.
- `mag serve <DIR>`: simple file server for content addressed data. The server is written in Rust, so is reasonably fast.
- `mag add <FILE>`: add content addressed data from a file. This command will create a new file in the working directory who's name is the CID and who's contents is the file bytes.

See `mag --help` for a full list of commands and features.

## TODO

- [x] `link` - create magnet for any HTTP URL
  - [x] Fetch URL
  - [x] Generate CID
  - [x] Integrity check
  - [ ] Generates infohash
  - [x] Return magnet
- [ ] `get` - fetch file using redundant sources
  - [x] Fetch magnet using http
  - [ ] Fetch magnet using BitTorrent
- [x] `add` - add content-addressed data to directory
- [ ] `serve` - content-addressed file server
  - [x] GET content by CID
  - [x] Store-and-forward federation
  - [x] POST content to server
  - [x] Option to turn off POST
  - [ ] Option to require secret to POST
  - [ ] Option to encrypt content ala Magenc
  - [x] Logging (tracing)
- [ ] Simple gossip-based federation
  - [x] Select n random peers and gossip updates to them
  - [x] Configurable list of peers to notify
  - [ ] Allow peer discovery (save peers to notify list when accepting notifications)
  - [x] Configurable peer deny list
  - [x] Configurable peer allow list
  - [ ] Configurable CID block list
- [ ] Peer bootstrapping API
  - [ ] Request all / as much data as possible from a peer in the form of CIDs
  - [ ] Kick off bootstrapping background task
-  [ ] JS library for parsing/fetching magnetized links

## Magnet links

Magnet links are used for locating data on BitTorrent. However, they are also a general-purpose protocol for bundling together multiple ways to fetch the same data. Magnetize extends magnet links, adding parameters to support content-addressed data over HTTP.

Magnet link parameters supported by Magnetize:

- `cid`: the CID.
- `cdn`: URL to a location where you can GET the data by CID. E.g. if `cdn=https://example.com/ipfs`, then you can GET `https://example.com/ipfs/<CID>`.
- `ws`: "Web Seed". A direct HTTP link to the data. Unlike `cdn`, this option does not require the source to conform to a URL format.
- `dn`: "Display Name". A suggested file name.
- `xt`: "Exact Topic". A BitTorrent infohash, allowing this magnet link to be used with BitTorrent clients.

While magnet links are a de facto standard, without a formal standardized, there have been [attempts to document the way BitTorrent clients commonly use magnet parameters](https://wiki.theory.org/BitTorrent_Magnet-URI_Webseeding). Magnetize aims to be compatible with these. That means you can construct hybrid magnet links that work with both Magnetize and [BitTorrent](https://blog.libtorrent.org/2020/09/bittorrent-v2/). Just include the `xt` parameter.

```url
magnet:?xt=urn:btmh:<INFOHASH>&cid=<CID>&cdn=https://example.com/ipfs
```

This gives the magnet link added resiliency by allowing clients to fall back to BitTorrent's p2p network when an HTTP source is unavailable. When used with BitTorrent, you can think of the `ws` and `cdn` parameters as high availability peers you can try first, while falling back to BitTorrent's DHT if they go down.

> TODO see if we can bundle a Torrent client into `mag get`.

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
