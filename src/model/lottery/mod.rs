use azure_data_cosmos::CosmosEntity;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserLottery {
    pub id: String,
    pub user_id: String,
    pub next_daily_time: String,
    pub next_weekly_time: String,
    pub lotteries: Vec<Vec<u8>>,
}

impl CosmosEntity for UserLottery {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}

impl Default for UserLottery {
    fn default() -> Self {
        UserLottery {
            id: "".to_string(),
            user_id: "".to_string(),
            next_daily_time: OffsetDateTime::UNIX_EPOCH
                .format(&Rfc3339)
                .unwrap_or_default(),
            next_weekly_time: OffsetDateTime::UNIX_EPOCH
                .format(&Rfc3339)
                .unwrap_or_default(),
            lotteries: vec![],
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UserLotteryUpdateInfo {
    pub lotteries: Vec<Vec<u8>>,
}
