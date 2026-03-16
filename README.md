# pager

## compile backend

cargo build --release --target x86_64-unknown-linux-gnu

## networking

- Serve frontend via github pages
- Connect to backend via port 443
- Host backend on OCI
- Backend sends push to apple servers
- Apple servers send push to iOS devices
- we need to talk to the backend via https since the frontend is also secured

## logging

View all logs for the rust backend: journalctl -u rust-backend

## useful commands

Connect to OCI
ssh -i "ssh-key-oci-2026-02-07.key" opc@132.226.217.85
