# magneto

"We have IPFS at home".

Decentralize HTTP somewhat with content addressed data.

## TODO

- [ ] `link` - create magnet for any HTTP URL
  - [ ] Fetch URL
  - [ ] Generate CID
  - [ ] Generates infohash
  - [ ] Return magnet
- [ ] `get` - fetch file using redundant sources
  - [ ] Fetch magnet using http
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
