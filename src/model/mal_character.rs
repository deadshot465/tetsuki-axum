use azure_data_cosmos::CosmosEntity;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MalCharacter {
    #[serde(rename = "Id")]
    pub character_id: i32,
    #[serde(rename = "Url")]
    pub url: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "NameKanji")]
    pub name_kanji: String,
    #[serde(rename = "ImageUrl")]
    pub image_url: String,
    #[serde(rename = "CreatedAt")]
    pub created_at: String,
    #[serde(rename = "About")]
    pub about: String,
    pub id: String,
}

impl CosmosEntity for MalCharacter {
    type Entity = i32;

    fn partition_key(&self) -> Self::Entity {
        self.character_id
    }
}
