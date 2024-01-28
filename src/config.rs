use envconfig::{Envconfig};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "M17WEB_PROXY_CALLSIGN", default = "N0CALL")]
    pub callsign: String,
    #[envconfig(from = "M17WEB_PROXY_LISTENER",default = "0.0.0.0:3000")]
    pub ws_listener_address: String,
    #[envconfig(from = "M17WEB_PROXY_REFLECTOR",default = "localhost:17000")]
    pub reflector_address: String,
    #[envconfig(from = "M17WEB_PROXY_MODULE",default = "A")]
    pub reflector_target_module: String,
}
