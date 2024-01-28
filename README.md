# m17web-proxy
- Connects to a module on a M17 reflector
- Provides websocket endpoint for the webclient
- Sends decoded source callsign and codec2 stream to all connected websocket clients
- Ideally this whole function should be implemented directly into the reflector
- Codec2 decoding should be handled in the client
- Test client in static folder

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
docker run -d --name m17web-proxy \
  -e M17WEB_PROXY_CALLSIGN=N0CALL \
  -e M17WEB_PROXY_LISTENER=localhost:17000
```
