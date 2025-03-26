
## Configuration

### Environment variables:
| Variable                  | Description                                                      | Default                 |
|---------------------------|------------------------------------------------------------------|-------------------------|
| M17WEB_PROXY_CALLSIGN     | Callsign of the proxy                                            | SWLXXXXX                |
| M17WEB_PROXY_LISTENER     | Address:Port to listen on                                        | 0.0.0.0:3000            |
| M17WEB_PROXY_SUBSCRIPTION | Format is *Designator*\_*Modules*\,*Designator*\_*Modules*\, ... | M17-XOR_ABC,M17-DEV_DEF | 

### Docker
```
docker build -t m17web-proxy .

docker run -dp 3000:3000 --name m17web-proxy \
  -e M17WEB_PROXY_SUBSCRIPTION=M17-XOR_ABC,M17-DEV_DEF \
  m17web-proxy
```

### Build

Tested on Ubuntu 22.04

Install dependencies:
```bash
sudo apt update && sudo apt install curl build-essential pkg-config libssl-dev -y
curl https://sh.rustup.rs -sSf | bash -s -- -y
PATH="$HOME/.cargo/bin:${PATH}"
```

Clone and build:
```bash
git clone https://github.com/OE3ANC/m17web-proxy
cd m17web-proxy
cargo build
./target/debug/m17web-proxy
```
