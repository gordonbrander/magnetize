# magnetize

> "We have IPFS at home"

Content-addressed data over HTTP using magnet links.

```url
magnet:?cid=<CID>&cdn=<URL>&cdn=<URL>
```

Minimum viable decentralization is n > 1. Magnetize achieves minimum viable decentralization by combining a CID with multiple redundant places to GET it over HTTP.

Notably, you do not have to trust the servers listed in the magnet link to serve the correct data. The CID acts as a cryptographic proof for the data, ensuring you get exactly the data you requested.

Magnetize offers a CLI with several tools for content-addressed data over HTTP:

- `mag get <MAGNET_URL>` - fetch content addressed over HTTP using a magnet link. This command will try locations until it finds one that succeeds.
- `mag serve <DIR>` - serves content addressed file data over HTTP. Files are served from the directory specified by the `dir` argument. The server is built in Rust, so is reasonably fast.
- `mag add <FILE>` - add content addressed data from a file. This command will create a new file in the current directory, whose name is the CID of the data.

See `mag --help` for a full list of commands and features.

## TODO

- [ ] `link` - create magnet for any HTTP URL
  - [ ] Fetch URL
  - [ ] Generate CID
  - [ ] Generates infohash
  - [ ] Return magnet
- [ ] `get` - fetch file using redundant sources
  - [x] Fetch magnet using http
  - [ ] Fetch magnet using BitTorrent
- [x] `add` - add content-addressed data to directory
- [ ] `mirror` - fetch content from URL and add it to directory
- [ ] `serve` - content-addressed file server
  - [x] GET content by CID
  - [x] POST content to server
  - [x] Option to turn of POST
  - [ ] Option to require secret to POST
  - [ ] Option to encrypt content ala Magenc
  - [ ] Simple federation using a text file containing a list of known CDNs (and maybe BitTorrent)
- [ ] JS library for parsing/fetching magnetized links

## Magnet links

Magnet links bundle together multiple ways to fetch the same data. They are frequently used for locating data on BitTorrent, but are a general-purpose protocol that can be used in various contexts. Magnetize extends magnet links, adding parameters to support content-addressed data over HTTP.

While magnet links are a de facto standard, with no formally standardized semantics,there have been [attempts to coordinate the way BitTorrent clients use parameters](https://wiki.theory.org/BitTorrent_Magnet-URI_Webseeding). Magnetize tries to remain broadly compatible with these efforts.

Magnet link parameters supported by Magnetize:

- `cid`: the CID.
- `cdn`: URL to a location where you can GET the data by CID. E.g. if `cdn=https://example.com/ipfs`, then you can GET `https://example.com/ipfs/<CID>`.
- `ws`: "Web Seed". A direct HTTP link to the data. Unlike `cdn`, this option does not require the source to conform to a URL format.
- `dn`: "Display Name". A suggested file name.
- `xt`: "Exact Topic". A BitTorrent infohash, allowing this magnet link to be used with BitTorrent clients.

Note it is possible to construct a hybrid magnet link that works with both Magnetize and [BitTorrent](https://blog.libtorrent.org/2020/09/bittorrent-v2/) by including the `xt` parameter.

```url
magnet:?xt=urn:btmh:<INFOHASH>&cid=<CID>&cdn=https://example.com/ipfs
```

This gives the magnet link added resiliency by allowing clients to fall back to BitTorrent's p2p network when an HTTP source is unavailable. When used with BitTorrent, you can think of the `ws` and `cdn` parameters as high availability peers you can try first, while falling back to BitTorrent's DHT if they go down.

> TODO see if we can bundle a Torrent client into `mag get`.

## CIDs

Magnetize supports one kind of IPFS CID:

- CIDv1
- base32 (multibase)
- sha256 (multihash)
- raw bytes (multicodec)

This CID type is described in more detail here: [dasl.ing/cid.html](dasl.ing/cid.html).

## Development

### Installing binaries on your path with Cargo

From the project directory:

```bash
cargo install --path .
```

This will install the binaries to `~/.cargo/bin`, which is usually added to your path by the Rust installer.
