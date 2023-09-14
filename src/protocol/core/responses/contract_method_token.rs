use primitive_types::H160;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ContractMethodToken {
    hash: H160,
    method: String,
    param_count: u32,
    has_return_value: bool,
    call_flags: String
}

impl ContractMethodToken {
    pub fn new(hash: H160, method: String, param_count: u32, has_return_value: bool, call_flags: String) -> Self {
        Self {
            hash,
            method,
            param_count,
            has_return_value,
            call_flags
        }
    }
}