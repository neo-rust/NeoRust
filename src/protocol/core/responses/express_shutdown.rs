use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct ExpressShutdown {
    #[serde(rename = "process-id")]
    process_id: i32
}

impl ExpressShutdown {
    pub fn new(process_id: i32) -> Self {
        Self { process_id }
    }
}