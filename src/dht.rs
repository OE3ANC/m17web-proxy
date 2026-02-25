use std::ffi::{CStr, CString};

use log::{error, info};
use serde::Deserialize;
use tokio::sync::oneshot;

/// Reflector configuration data from the ham-dht network.
/// Field order matches the C++ MSGPACK_DEFINE in SMrefdConfig1.
#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct MrefdConfig {
    pub timestamp: i64,
    pub callsign: String,
    pub ipv4addr: String,
    pub ipv6addr: String,
    pub modules: String,
    pub encryptedmods: String,
    pub url: String,
    pub email: String,
    pub sponsor: String,
    pub country: String,
    pub version: String,
    pub port: u16,
}

/// Value ID for the Config section of an mrefd document on the DHT.
const MREFD_CONFIG_VALUE_ID: u64 = 1;
/// Expected user_type string for mrefd config values.
const MREFD_CONFIG_USER_TYPE: &str = "mrefd-config-1";

/// A safe wrapper around the OpenDHT DhtRunner.
pub struct DhtNode {
    runner: *mut opendht_sys::dht_runner,
}

// The DhtRunner is thread-safe (it manages its own internal threading)
unsafe impl Send for DhtNode {}
unsafe impl Sync for DhtNode {}

/// Data collected during a DHT get callback.
struct GetCallbackData {
    /// The best (most recent) config found so far.
    best_config: Option<MrefdConfig>,
    /// Completion signal sender.
    done_tx: Option<oneshot::Sender<Option<MrefdConfig>>>,
}

/// C callback invoked for each value found during a `dht_runner_get`.
/// Returns `true` to continue receiving values.
unsafe extern "C" fn get_value_callback(
    value: *const opendht_sys::dht_value,
    user_data: *mut libc::c_void,
) -> bool {
    if value.is_null() || user_data.is_null() {
        return true;
    }

    let data = &mut *(user_data as *mut GetCallbackData);

    // Check the value ID - we only want Config (id=1)
    let value_id = opendht_sys::dht_value_get_id(value);
    if value_id != MREFD_CONFIG_VALUE_ID {
        return true;
    }

    // Check the user_type string
    let user_type_ptr = opendht_sys::dht_value_get_user_type(value);
    if user_type_ptr.is_null() {
        return true;
    }
    let user_type = CStr::from_ptr(user_type_ptr);
    if user_type.to_str().unwrap_or("") != MREFD_CONFIG_USER_TYPE {
        return true;
    }

    // Get the raw msgpack data
    let data_view = opendht_sys::dht_value_get_data(value);
    if data_view.data.is_null() || data_view.size == 0 {
        return true;
    }

    let raw_data = std::slice::from_raw_parts(data_view.data, data_view.size);

    // Deserialize the msgpack data into MrefdConfig
    match rmp_serde::from_slice::<MrefdConfig>(raw_data) {
        Ok(config) => {
            // Keep the most recent config (highest timestamp)
            let dominated = data
                .best_config
                .as_ref()
                .is_some_and(|existing| existing.timestamp >= config.timestamp);
            if !dominated {
                info!(
                    "DHT: Found config for {} (v{}) at {}:{} (ts={})",
                    config.callsign, config.version, config.ipv4addr, config.port, config.timestamp
                );
                data.best_config = Some(config);
            }
        }
        Err(e) => {
            error!("DHT: Failed to deserialize mrefd config: {}", e);
        }
    }

    true
}

/// C callback invoked when the `dht_runner_get` operation completes.
unsafe extern "C" fn get_done_callback(ok: bool, user_data: *mut libc::c_void) {
    if user_data.is_null() {
        return;
    }

    let data = Box::from_raw(user_data as *mut GetCallbackData);

    if !ok {
        error!("DHT: get() operation failed");
    }

    if let Some(tx) = data.done_tx {
        let _ = tx.send(data.best_config);
    }
}

impl DhtNode {
    /// Create a new DHT node and start it on the given port with a generated identity.
    pub fn new(port: u16, identity_name: &str) -> Result<Self, String> {
        unsafe {
            let runner = opendht_sys::dht_runner_new();
            if runner.is_null() {
                return Err("Failed to create DHT runner".to_string());
            }

            // Generate a crypto identity for this node
            let name_c = CString::new(identity_name).map_err(|e| e.to_string())?;
            let identity = opendht_sys::dht_identity_generate(name_c.as_ptr(), std::ptr::null());

            // Configure the runner
            let mut config: opendht_sys::dht_runner_config = std::mem::zeroed();
            opendht_sys::dht_runner_config_default(&mut config);
            // Network 59973 is the ham-dht network used by mrefd reflectors
            config.dht_config.node_config.network = 59973;
            config.dht_config.node_config.is_bootstrap = false;
            config.dht_config.node_config.maintain_storage = false;
            config.dht_config.id = identity;
            config.threaded = true;
            config.peer_discovery = false;
            config.peer_publish = false;

            let result = opendht_sys::dht_runner_run_config(runner, port, &config);
            if result != 0 {
                opendht_sys::dht_runner_delete(runner);
                return Err(format!("Failed to start DHT runner (error code: {})", result));
            }

            Ok(DhtNode { runner })
        }
    }

    /// Bootstrap this node into the ham-dht network.
    pub fn bootstrap(&self, host: &str, port: &str) {
        let host_c = CString::new(host).expect("Invalid bootstrap host");
        let port_c = CString::new(port).expect("Invalid bootstrap port");

        unsafe {
            opendht_sys::dht_runner_bootstrap(self.runner, host_c.as_ptr(), port_c.as_ptr());
        }

        info!("DHT: Bootstrapping to {}:{}", host, port);
    }

    /// Query the DHT for a reflector's configuration.
    /// The designator should be the full reflector callsign, e.g. "M17-XOR".
    pub async fn get_reflector_config(
        &self,
        designator: &str,
    ) -> Result<MrefdConfig, String> {
        let (tx, rx) = oneshot::channel();

        let callback_data = Box::new(GetCallbackData {
            best_config: None,
            done_tx: Some(tx),
        });
        let callback_data_ptr = Box::into_raw(callback_data) as *mut libc::c_void;

        // Compute the InfoHash from the designator string
        let designator_upper = designator.to_uppercase();
        let designator_bytes = designator_upper.as_bytes();

        unsafe {
            let mut hash: opendht_sys::dht_infohash = std::mem::zeroed();
            opendht_sys::dht_infohash_get(
                &mut hash,
                designator_bytes.as_ptr(),
                designator_bytes.len(),
            );

            info!(
                "DHT: Querying for {} (hash: {})",
                designator_upper,
                CStr::from_ptr(opendht_sys::dht_infohash_print(&hash))
                    .to_str()
                    .unwrap_or("?")
            );

            opendht_sys::dht_runner_get(
                self.runner,
                &hash,
                Some(get_value_callback),
                Some(get_done_callback),
                callback_data_ptr,
            );
        }

        // Wait for the get operation to complete
        match rx.await {
            Ok(Some(config)) => Ok(config),
            Ok(None) => Err(format!(
                "No configuration found on DHT for {}",
                designator
            )),
            Err(_) => Err(format!(
                "DHT get operation channel closed for {}",
                designator
            )),
        }
    }
}

impl Drop for DhtNode {
    fn drop(&mut self) {
        unsafe {
            opendht_sys::dht_runner_delete(self.runner);
        }
    }
}

/// Resolve a reflector designator to an address string (ip:port) using the DHT.
pub async fn get_ref_address_from_dht(
    dht_node: &DhtNode,
    designator: &str,
) -> Result<String, String> {
    let config = dht_node.get_reflector_config(designator).await?;

    if config.ipv4addr.is_empty() {
        return Err(format!("Reflector {} has no IPv4 address on DHT", designator));
    }

    Ok(format!("{}:{}", config.ipv4addr, config.port))
}
