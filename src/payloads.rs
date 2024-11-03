use crate::utils::encode_callsign;

pub fn create_conn_payload(callsign: String, module: String) -> Vec<u8> {
    let mut payload = vec![];
    payload.extend_from_slice("LSTN".as_bytes());
    payload.extend_from_slice(encode_callsign(callsign).as_slice());
    payload.extend_from_slice(module.as_bytes());
    payload
}

pub fn create_pong_payload(callsign: String) -> Vec<u8> {
    let mut payload = vec![];
    payload.extend_from_slice("PONG".as_bytes());
    payload.extend_from_slice(encode_callsign(callsign.clone()).as_slice());
    payload
}
