use envconfig::{Envconfig};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "M17WEB_PROXY_CALLSIGN", default = "M17WEB")]
    pub callsign: String,
    #[envconfig(from = "M17WEB_PROXY_LISTENER", default = "0.0.0.0:3000")]
    pub ws_listener_address: String,
    #[envconfig(from = "M17WEB_PROXY_SUBSCRIPTION", default = "M17-XOR_ABC")]
    pub subscription: String,
}
