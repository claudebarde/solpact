use serde::{Deserialize, Serialize};
use serde_json;
use solang_parser::pt::{ContractPart, Expression, SourceUnitPart};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAbi {
    pub ledger: Vec<LedgerValue>,
    pub witnesses: Vec<Witness>,
    pub circuits: Vec<CircuitSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerValue {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub visibility: Option<String>, // optional if not always present
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitSignature {
    pub name: String,
    pub inputs: Vec<Param>,
    pub outputs: Param,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

fn build_abi(ast: &SourceUnitPart) -> Result<String, String> {
    // let abi: BuildAbi = serde_json::from_str(&json_text)?;
    // let json_text = serde_json::to_string_pretty(&abi)?;

    Ok("test".to_string())
}
