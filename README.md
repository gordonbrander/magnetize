# magnetize

> "We have IPFS at home".

Content addressed data over HTTP using magnet links.

Basically include a CID, plus multiple redundant places where you can GET it over HTTP.

```
magnet:?cid=<CID>&cdn=<URL>&cdn=<URL>
```


## Magnet links

Magnet links bundle together multiple ways to fetch the same data. They are frequently used for locating data on BitTorrent, but are a general-purpose protocol that can be used in various contexts. Magnetize extends magnet links, adding parameters to support content-addressed data over HTTP.

Supported magnet link parameters:

- `cid`: the CID.
- `cdn`: URL to a location where you can GET the data by CID. E.g. if `cdn=https://example.com/ipfs`, then you can GET `https://example.com/ipfs/<CID>`.
- `xs`: "Exact Source". A direct HTTP link to the data. Unlike `cdn`, this option does not require the source to conform to a URL format.
- `xt`: "Exact Topic". The BitTorrent infohash (optional).
- `dn`: "Display Name". A suggested file name.

## CIDs

Magnetize supports one kind of IPFS CID:

- CIDv1
- base32 (multibase)
- sha256 (multihash)
- raw bytes (multicodec)

This CID type is described in more detail here: [dasl.ing/cid.html](dasl.ing/cid.html).

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

## Development

### Installing binaries on your path with Cargo

From the project directory:

```bash
cargo install --path .
```

This will install the binaries to `~/.cargo/bin`, which is usually added to your path by the Rust installer.
