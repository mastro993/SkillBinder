//! Cursor's MCP install link: the server's name and its `mcp.json` entry as base64 JSON, which
//! Cursor shows the user to accept (<https://cursor.com/docs/mcp/install-links>).

use std::path::Path;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The link that asks Cursor to add `name`, running `command mcp`.
pub(super) fn cursor(name: &str, command: &Path) -> String {
    let server = serde_json::json!({ "command": command.to_string_lossy(), "args": ["mcp"] });
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("name", name)
        .append_pair("config", &base64(server.to_string().as_bytes()))
        .finish();
    format!("cursor://anysphere.cursor-deeplink/mcp/install?{query}")
}

/// Standard base64 with padding.
pub(super) fn base64(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let [a, b, c] = [0, 1, 2].map(|index| chunk.get(index).copied().unwrap_or(0));
        let bits = u32::from(a) << 16 | u32::from(b) << 8 | u32::from(c);
        for (position, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if position <= chunk.len() {
                encoded.push(char::from(ALPHABET[(bits >> shift & 0x3f) as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}
