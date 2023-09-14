use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct NeoGetStateHeight {
    pub state_height: Option<StateHeight>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct StateHeight {
    #[serde(rename = "localrootindex")]
    pub local_root_index: u32,
    #[serde(rename = "validatedrootindex")]
    pub validated_root_index: u32,
}