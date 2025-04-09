A rust-based client and file server for decentralizing HTTP.

## CLI

```sh
mag get magnet:?xt=urn:sha256d:<SHA256>&xs=<URL1>&xs=<URL2>&dn=file.txt&sig=ed25519:<Base64-URL-ED25519-Signature>&did=did:key:abc123
```

Options:
- `-dn`: Display name, a suggested name for the file
- `-h` or `--help`: For help and a full set of options

## Server

```
mag serve ./public/
```

Serves content-addressed data from the specified directory.

Data is saved/retreived from flat files stored by content address in the specified directory.

## Links

```
magnet:?xt=urn:sha256d:<SHA256>&xs=<URL1>&xs=<URL2>&dn=file.txt&sig=ed25519:<Base64-URL-ED25519-Signature>&did=did:key:abc123
```

- `xt=urn:sha256d:<SHA256>`: Exact topic (standard in magnet links)
    - urn is the hash over the file bytes
    - (sha256d is sha256 applied twice, to avoid length extension attacks)
- `xs=<URL>`: Exact source (standard in magnet links).
    - Multiple `xs` entries allowed for redundant retrieval (multiple mirrors/servers/CDNs).
- `dn=file.txt`: Display name, a suggested name for the file
- `sig=ed25519:<Base64-URL-ED25519-Signature>` a prefix describing the crypto suite, followed by the signature over the file bytes
    - Only ed25519 is expected to be supported at this time
- `did=<DID>` did resolving to the public key of the
    - Only `did:key` is expected to be supported at this time
