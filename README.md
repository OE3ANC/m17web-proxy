
## Configuration

### Environment variables:
| Variable | Description                                                      | Default                 |
| -------- |------------------------------------------------------------------|-------------------------|
| M17WEB_PROXY_CALLSIGN | Callsign of the proxy                                            | M17WEB                  |
| M17WEB_PROXY_LISTENER | Address:Port to listen on                                        | 0.0.0.0:3000            |
| M17WEB_PROXY_SUBSCRIPTION | Format is *Designator*\_*Modules*\,*Designator*\_*Modules*\, ... | M17-XOR_ABC,M17-DEV_DEF | 

At the moment it's only possible to proxy a single module. This will change in the future!

### Docker
```
docker build -t m17web-proxy .

docker run -dp 3000:3000 --name m17web-proxy \
  -e M17WEB_PROXY_CALLSIGN=M17WEB \
  -e M17WEB_PROXY_SUBSCRIPTION=M17-XOR_ABC,M17-DEV_DEF \
  m17web-proxy
```
