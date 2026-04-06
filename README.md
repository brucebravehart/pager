# pager

## Backend Build

Build the Linux release binary with:

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

## Deployment Notes

- Frontend is served from GitHub Pages.
- Backend runs on OCI and listens on port 443.
- The frontend talks to the backend over HTTPS because the app is served securely.
- Push notifications flow through Apple push services to iOS devices.

## Logging

Use the following command to inspect the Rust backend logs:

```bash
journalctl -u rust-backend
```

## OCI Access

```bash
ssh -i "ssh-key-oci-2026-02-07.key" opc@132.226.217.85
```
