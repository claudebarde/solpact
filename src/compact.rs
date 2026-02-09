use serde::Deserialize;

pub fn sol_to_compact_type(sol_type: &str) -> Option<CompactType> {
    if let Some(bits) = sol_type.strip_prefix("uint") {
        let size = if bits.is_empty() { "256" } else { bits };
        if size.chars().all(|c| c.is_ascii_digit()) {
            return Some(CompactType::Uint(size.parse().unwrap()));
        }
    }

    if let Some(bits) = sol_type.strip_prefix("bytes") {
        let size = if bits.is_empty() { "256" } else { bits };
        if size.chars().all(|c| c.is_ascii_digit()) {
            return Some(CompactType::Bytes(size.parse().unwrap()));
        }
    }

    match sol_type {
        "bool" => Some(CompactType::Boolean),
        "bytes32" => Some(CompactType::Bytes(32)),
        "Counter" => Some(CompactType::Counter),
        "MaybeOpString" => Some(CompactType::Maybe(Box::new(CompactType::OpaqueString))),
        "string" => Some(CompactType::OpaqueString),
        _ => None,
    }
}

pub fn format_sol_to_compact_type(compound: Vec<String>) -> Option<CompactType> {
    if compound.len() == 2 && compound[0] == "CSL" {
        // Handles types from the CompactStandardLibrary (e.g., CSL.MaybeOpString)
        return sol_to_compact_type(&compound[1]);
    } else {
        return None;
    }
}

pub fn csl_member_access(member: &str) -> Option<String> {
    match member {
        "noneOpString" => Some("none".to_string()),
        "some" => Some("some".to_string()),
        "persistentHash" => Some("persistentHash".to_string()),
        "pad32" => Some("pad32".to_string()),
        _ => None,
    }
}

pub fn compact_member_access(member: &str) -> Option<String> {
    match member {
        "disclose" => Some("disclose".to_string()),
        _ => None,
    }
}

pub fn compact_pad(sol_name: &str, str_to_pad: &str) -> String {
    match sol_name {
        "pad32" => format!("pad(32, {})", str_to_pad),
        _ => panic!("Unknown pad function: {}", sol_name),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum CompactType {
    Void,
    Uint(usize),
    Bytes(usize),
    Boolean,
    OpaqueString,
    Maybe(Box<CompactType>),
    Counter,
    Enum((String, Vec<String>)), // (name, variants)
}
impl CompactType {
    pub fn from_string(s: &str) -> Option<CompactType> {
        // Implementation to convert from string to CompactType
        match s {
            "bool" => Some(CompactType::Boolean),
            "Opaque<\"string\">" => Some(CompactType::OpaqueString),
            _ if s.starts_with("Uint<") && s.ends_with('>') => {
                let size_str = &s[5..s.len() - 1];
                if let Ok(size) = size_str.parse::<usize>() {
                    Some(CompactType::Uint(size))
                } else {
                    None
                }
            }
            _ if s.starts_with("Bytes<") && s.ends_with('>') => {
                let size_str = &s[6..s.len() - 1];
                if let Ok(size) = size_str.parse::<usize>() {
                    Some(CompactType::Bytes(size))
                } else {
                    None
                }
            }
            _ if s.starts_with("Maybe<") && s.ends_with('>') => {
                let inner_type = &s[6..s.len() - 1];
                if let Some(inner_compact_type) = CompactType::from_string(inner_type) {
                    Some(CompactType::Maybe(Box::new(inner_compact_type)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            CompactType::Void => "[]".to_string(),
            CompactType::Uint(size) => format!("Uint<{}>", size),
            CompactType::Bytes(size) => format!("Bytes<{}>", size),
            CompactType::Boolean => "Boolean".to_string(),
            CompactType::OpaqueString => "Opaque<\"string\">".to_string(),
            CompactType::Maybe(inner_type) => {
                format!("Maybe<{}>", inner_type.to_string())
            }
            CompactType::Counter => "Counter".to_string(),
            CompactType::Enum((name, _)) => {
                format!("{}", name)
            }
        }
    }
}
