# m17web-proxy
- Connects to a module on a M17 reflector
- Provides websocket endpoint for the webclient
- Sends decoded source callsign and codec2 stream to all connected websocket clients
- Ideally this whole function should be implemented directly into the reflector
- Codec2 decoding should be handled in the client
- Test client in static folder
- Change reflector IP and Callsign in main.rs