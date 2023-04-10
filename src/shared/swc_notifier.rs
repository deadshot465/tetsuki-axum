use crate::model::swc::{DungeonType, SwcPushMessage};
use crate::shared::configuration::CONFIGURATION;
use crate::shared::HTTP_CLIENT;
use std::ops::Add;
use time::OffsetDateTime;
use time::Weekday::{Friday, Thursday};

pub async fn initialize_tartarus_notification() {
    let now = OffsetDateTime::now_utc();
    let midnight = now.date().midnight();
    let mut next_datetime = midnight.add(time::Duration::hours(22));
    let mut weekday = next_datetime.weekday();
    while weekday != Friday {
        next_datetime = next_datetime.add(time::Duration::days(1));
        weekday = next_datetime.weekday();
    }
    schedule_notification(next_datetime.assume_utc(), DungeonType::Tartarus).await;
}

pub async fn initialize_slime_notification() {
    let now = OffsetDateTime::now_utc();
    let midnight = now.date().midnight();
    let mut next_datetime = midnight.add(time::Duration::hours(16));
    let mut weekday = next_datetime.weekday();
    while weekday != Thursday {
        next_datetime = next_datetime.add(time::Duration::days(1));
        weekday = next_datetime.weekday();
    }
    schedule_two_step_notification(
        next_datetime.assume_utc(),
        DungeonType::SlimePhaseOne,
        DungeonType::SlimePhaseTwo,
    )
    .await;
}

async fn schedule_notification(mut next_day: OffsetDateTime, dungeon_type: DungeonType) {
    loop {
        let duration = next_day - OffsetDateTime::now_utc();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs_f32(
            duration.as_seconds_f32(),
        ));
        interval.tick().await;
        publish_notification(dungeon_type).await;
        next_day = next_day.add(time::Duration::days(7));
    }
}

async fn schedule_two_step_notification(
    mut next_day: OffsetDateTime,
    first_dungeon_type: DungeonType,
    second_dungeon_type: DungeonType,
) {
    loop {
        let duration = next_day - OffsetDateTime::now_utc();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs_f32(
            duration.as_seconds_f32(),
        ));
        interval.tick().await;
        publish_notification(first_dungeon_type).await;
        next_day = next_day.add(time::Duration::hours(12));
        let duration = next_day - OffsetDateTime::now_utc();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs_f32(
            duration.as_seconds_f32(),
        ));
        interval.tick().await;
        publish_notification(second_dungeon_type).await;
        next_day = next_day.add(time::Duration::days(7));
    }
}

async fn publish_notification(dungeon_type: DungeonType) {
    let payload = SwcPushMessage::DungeonNotification { dungeon_type };

    for endpoint in CONFIGURATION.swc_publication_endpoints.iter() {
        if let Err(e) = HTTP_CLIENT.post(endpoint).json(&payload).send().await {
            tracing::error!("Failed to publish dungeon notification: {}", e);
        }
    }
}
