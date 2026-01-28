pub fn sol_to_compact_type(sol_type: &str) -> Option<String> {
    if let Some(bits) = sol_type.strip_prefix("uint") {
        let size = if bits.is_empty() { "256" } else { bits };
        if size.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("Uint<{}>", size));
        }
    }

    if let Some(bits) = sol_type.strip_prefix("bytes") {
        let size = if bits.is_empty() { "256" } else { bits };
        if size.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("Bytes<{}>", size));
        }
    }

    match sol_type {
        "bool" => Some("Boolean".to_string()),
        _ => None,
    }
}
