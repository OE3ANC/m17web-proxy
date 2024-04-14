use rand::Rng;
use random_string::generate;

// Base40 charset
pub const CHARSET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/.";

// Base40 decode
pub fn decode_callsign(encoded: &[u8]) -> String {
    assert!(!encoded.is_empty());

    let callsign: &mut String = &mut "".to_string();

    if encoded[..6].iter().all(|&b| b == 0xFF) {
        *callsign = "ALL".to_owned()
    }

    let mut enc: u64 = (encoded[0] as u64) << 40
        | (encoded[1] as u64) << 32
        | (encoded[2] as u64) << 24
        | (encoded[3] as u64) << 16
        | (encoded[4] as u64) << 8
        | (encoded[5] as u64);

    if enc >= 262144000000000 { // 40^9
        *callsign = "".to_owned()
    }

    let digits = CHARSET.to_string().into_bytes();
    while enc > 0 {
        let digit = digits[(enc % 40) as usize];
        callsign.push(char::from(digit));
        enc /= 40;
    }
    callsign.to_owned()
}

// Base40 encode
pub fn encode_callsign(callsign: String) -> Vec<u8> {
    let encoded: &mut [u8] = &mut [0;6];

    if callsign == "ALL" || callsign == " ALL      " {
        encoded[..].fill(0xFF)
    }

    let len = callsign.trim().len().min(9);

    let mut enc = 0;
    for ch in callsign[..len].chars().rev() {
        let pos = CHARSET
            .chars()
            .position(|c| c == ch)
            .unwrap_or(0) as u64;

        enc *= 40;
        enc += pos;
    }

    encoded[0] = ((enc >> 40) & 0xff) as u8;
    encoded[1] = ((enc >> 32) & 0xff) as u8;
    encoded[2] = ((enc >> 24) & 0xff) as u8;
    encoded[3] = ((enc >> 16) & 0xff) as u8;
    encoded[4] = ((enc >>  8) & 0xff) as u8;
    encoded[5] = ((enc >>  0) & 0xff) as u8;

    encoded.to_owned()
}

pub fn generate_lstn_call() -> String {
    let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let prefix = "SWL".to_string();
    format!("{prefix}{}", generate(6, charset))
}