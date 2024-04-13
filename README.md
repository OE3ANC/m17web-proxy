
## Configuration

### Environment variables:
| Variable | Description | Default         |
| -------- | ----------- |-----------------|
| M17WEB_PROXY_CALLSIGN | Callsign of the proxy | N0CALL          |
| M17WEB_PROXY_LISTENER | Address to listen on | 0.0.0.0:3000    |
| M17WEB_PROXY_REFLECTOR | Address of the reflector | localhost:17000 | 
| M17WEB_PROXY_MODULE | Module to connect to | A               |

### Docker
```
docker build -t m17web-proxy .

docker run -dp 3000:3000 --name m17web-proxy \
  -e M17WEB_PROXY_CALLSIGN=N0CALL \
  -e M17WEB_PROXY_REFLECTOR=localhost:17000 m17web-proxy
```
