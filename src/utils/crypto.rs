use anyhow::{Ok, Result};
use base64::prelude::*;
pub fn encode_b64(input: &str) -> String {
    BASE64_STANDARD.encode(input)
}
pub fn decode_b64(input: &str) -> Result<String> {
    let utf8o = BASE64_STANDARD.decode(input)?;
    let s = String::from_utf8(utf8o)?;
    Ok(s)
}
