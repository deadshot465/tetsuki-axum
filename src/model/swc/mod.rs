use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Payload {
    pub data: Vec<Coupon>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Coupon {
    #[serde(rename = "Created")]
    pub created: Created,
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "Resources")]
    pub resources: Vec<Resource>,
    #[serde(rename = "Score")]
    pub score: String,
    #[serde(rename = "Status")]
    pub status: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Created {
    pub full: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Resource {
    #[serde(rename = "Quantity")]
    pub quantity: String,
    #[serde(rename = "Sw_Resource")]
    pub sw_resource: SwResource,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SwResource {
    #[serde(rename = "Category")]
    pub category: String,
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "Label_I18n")]
    pub label_i18n: String,
    #[serde(rename = "Priority")]
    pub priority: String,
    #[serde(rename = "Usable_In_Coupon")]
    pub usable_in_coupon: String,
    #[serde(rename = "Visible")]
    pub visible: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocalizedCoupon {
    pub coupon_code: String,
    pub resources: Vec<LocalizedResource>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocalizedResource {
    pub quantity: i32,
    pub label: String,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub enum DungeonType {
    Tartarus,
    SlimePhaseOne,
    SlimePhaseTwo,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
#[serde(tag = "type")]
pub enum SwcPushMessage {
    DungeonNotification { dungeon_type: DungeonType },
}
