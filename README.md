# Pnyxtr: Nostr Wallet Connect to a Fedimint Client

(Fork of benthecarman/nostr-wallet-connect-lnd)

This lets you create a fedimint client controlled by nostr wallet connect uris.

## Install

```bash
cargo build --release
cargo install --path .
```

## Usage

```bash
pnyxtr --data_dir /absolute/path/to/your/data/dir --relay wss://relay.damus.io
```

This will print a wallet connect uri to the console. Scan this with your wallet connect enabled wallet.
You may need to use a tool to turn the uri into a QR code.

![Pnyxtr](assets/image.png)
