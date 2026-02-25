# m17web-proxy

M17 web proxy that connects to M17 reflectors and streams audio/data to WebSocket clients.

Reflector discovery uses the [ham-dht](https://github.com/n7tae/ham-dht-tools) distributed hash table network via [OpenDHT](https://github.com/savoirfairelinux/opendht) as the primary method. When a reflector is not found on the DHT, the proxy falls back to the [RefCheck.Radio](https://refcheck.radio) M17 host file for address resolution.

## Configuration

### Environment variables:
| Variable                    | Description                                                      | Default                                                  |
|-----------------------------|------------------------------------------------------------------|----------------------------------------------------------|
| M17WEB_PROXY_CALLSIGN       | Callsign of the proxy                                            | SWLXXXXX                                                 |
| M17WEB_PROXY_LISTENER        | Address:Port to listen on                                        | 0.0.0.0:3000                                             |
| M17WEB_PROXY_SUBSCRIPTION    | Format is *Designator*\_*Modules*\,*Designator*\_*Modules*\, ... | M17-XOR_ABC                                              |
| M17WEB_PROXY_DHT_BOOTSTRAP   | Bootstrap node for the ham-dht network                           | xrf757.openquad.net                                      |
| M17WEB_PROXY_DHT_PORT        | Port for the ham-dht bootstrap node                              | 17171                                                    |
| M17WEB_PROXY_HOSTFILE_URL    | URL for the M17 reflector host file (fallback)                   | https://hostfiles.refcheck.radio/M17Hosts.json           |
| RUST_LOG                     | Log level (e.g. `info`, `debug`, `warn`)                         | (unset — defaults to error)                              |

### Docker
```
docker build -t m17web-proxy .

docker run -dp 3000:3000 --name m17web-proxy \
  -e M17WEB_PROXY_SUBSCRIPTION=M17-XOR_ABC,M17-DEV_DEF \
  m17web-proxy
```

### Build

Tested on Ubuntu 24.04

Install dependencies:
```bash
sudo apt update && sudo apt install -y \
  curl build-essential pkg-config libssl-dev \
  cmake ninja-build \
  libgnutls28-dev libmsgpack-dev libargon2-dev \
  libasio-dev libfmt-dev nettle-dev \
  libclang-dev clang

curl https://sh.rustup.rs -sSf | bash -s -- -y
PATH="$HOME/.cargo/bin:${PATH}"
```

Clone and build:
```bash
git clone https://github.com/OE3ANC/m17web-proxy
cd m17web-proxy
cargo build --release
./target/release/m17web-proxy
```

## How it works

At startup, the proxy:
1. Initializes an OpenDHT node and bootstraps into the ham-dht network
2. Fetches the M17 host file from RefCheck.Radio as a fallback data source (in parallel with DHT bootstrap)
3. For each subscribed reflector (e.g., `M17-XOR`):
   - First tries to resolve the address via DHT lookup
   - If DHT lookup fails, falls back to the host file data
   - Logs a warning when DHT fails and fallback is used
4. Connects to each reflector module via UDP
5. Streams received M17 voice/data frames to connected WebSocket clients

The ham-dht network is a decentralized system where M17 reflectors publish their configuration directly. The host file from RefCheck.Radio serves as a fallback for reflectors that are not registered on the DHT or when the DHT bootstrap is not working.
