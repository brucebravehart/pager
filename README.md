# pager

## compile backend

cargo build --release --target x86_64-unknown-linux-gnu

## networking

- Serve frontend via github pages
- Connect to backend via port 443
- Host backend on OCI
- Backend sends push to apple servers
- Apple servers send push to iOS devices

## logging

View all logs for the rust backend: journalctl -u rust-backend
