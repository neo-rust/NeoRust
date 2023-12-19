use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessRule {
	pub action: WitnessAction,
	pub condition: WitnessCondition,
}

impl WitnessRule {
	pub fn new(action: WitnessAction, condition: WitnessCondition) -> Self {
		Self { action, condition }
	}
}
