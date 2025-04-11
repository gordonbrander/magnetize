# magneto

Content-addressed data over HTTP. "We have IPFS at home".

## TODO

- [ ] `link` - create magnet for any HTTP URL
  - [ ] Fetch URL
  - [ ] Generate CID
  - [ ] Generates infohash
  - [ ] Return magnet
- [ ] `get` - fetch file using redundant sources
  - [ ] Fetch magnet using http
  - [ ] Fetch magnet using BitTorrent
- [ ] `serve` - content-addressed file server
  - [x] GET content by CID
  - [x] POST content to server
  - [x] Option to turn of POST
  - [ ] Option to gate on secret
  - [ ] Option to encrypt ala Magenc

## Development

### Installing binaries on your path with Cargo

From the project directory:

```bash
cargo install --path .
```

This will install the binaries to `~/.cargo/bin`, which is usually added to your path by the Rust installer.
