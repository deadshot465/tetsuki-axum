use crate::model::swc::{LocalizedCoupon, LocalizedResource, Payload};
use crate::shared::configuration::CONFIGURATION;
use crate::shared::HTTP_CLIENT;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const SWC_COUPON_WEBSITE_URL: &str = "https://swq.jp/_special/rest/Sw/Coupon";

static RESOURCE_NAME_MAPPING: OnceCell<HashMap<String, String>> = OnceCell::new();

pub async fn initialize_scraper() {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        60 * 60 * CONFIGURATION.swc_check_interval as u64,
    ));
    let resource_stack = Arc::new(Mutex::new(vec![]));

    loop {
        interval.tick().await;
        if let Err(e) = scrap_swc_coupons(resource_stack.clone()).await {
            tracing::error!("Failed to scrap swc coupons: {}", e);
        }
    }
}

async fn scrap_swc_coupons(resource_stack: Arc<Mutex<Vec<LocalizedCoupon>>>) -> anyhow::Result<()> {
    let payload = reqwest::get(SWC_COUPON_WEBSITE_URL)
        .await?
        .json::<Payload>()
        .await?;

    let mut code_and_coupons = payload
        .data
        .into_iter()
        .filter(|coupon| coupon.status.as_str() == "verified")
        .map(|coupon| (coupon.label.clone(), coupon))
        .collect::<Vec<_>>();

    code_and_coupons.sort_by(|(_, coupon_1), (_, coupon_2)| {
        let coupon_1_created = coupon_1.created.full.parse::<u64>().unwrap_or_default();
        let coupon_2_created = coupon_2.created.full.parse::<u64>().unwrap_or_default();
        coupon_1_created.cmp(&coupon_2_created)
    });

    let localized_coupons = code_and_coupons
        .into_iter()
        .map(|(coupon_code, coupon)| {
            let resources = coupon
                .resources
                .into_iter()
                .map(|resource| LocalizedResource {
                    quantity: resource.quantity.parse().unwrap_or_default(),
                    label: RESOURCE_NAME_MAPPING
                        .get_or_init(initialize_name_mapping)
                        .get(&resource.sw_resource.label_i18n)
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();

            LocalizedCoupon {
                coupon_code,
                resources,
            }
        })
        .collect::<Vec<_>>();

    let localized_coupons = sanitize_coupons(resource_stack, localized_coupons);
    publish_coupons(localized_coupons).await;

    Ok(())
}

fn initialize_name_mapping() -> HashMap<String, String> {
    HashMap::from([
        ("Energy".into(), "能量".into()),
        ("Crystal".into(), "紅石".into()),
        ("Mana".into(), "藍石".into()),
        ("Mystical scroll".into(), "神秘召喚書".into()),
        ("Light and dark scroll".into(), "光暗召喚書".into()),
        ("Fire scroll".into(), "火屬性召喚書".into()),
        ("Water scroll".into(), "水屬性召喚書".into()),
        ("Wind scroll".into(), "風屬性召喚書".into()),
        ("Rune".into(), "符文".into()),
        ("Sommoning stones".into(), "特殊召喚".into()),
    ])
}

fn sanitize_coupons(
    resource_stack: Arc<Mutex<Vec<LocalizedCoupon>>>,
    incoming_coupons: Vec<LocalizedCoupon>,
) -> Vec<LocalizedCoupon> {
    if let Ok(mut published_coupons) = resource_stack.lock() {
        let coupons = incoming_coupons
            .into_iter()
            .filter(|coupon| {
                !published_coupons
                    .iter()
                    .any(|c| c.coupon_code.as_str() == coupon.coupon_code.as_str())
            })
            .collect::<Vec<_>>();
        published_coupons.clear();

        let mut coupons_clone = coupons.clone();
        published_coupons.append(&mut coupons_clone);

        coupons
    } else {
        incoming_coupons
    }
}

async fn publish_coupons(coupons: Vec<LocalizedCoupon>) {
    let mut swc_payload = HashMap::new();
    swc_payload.insert("swc_payload".to_string(), coupons);

    for endpoint in CONFIGURATION.swc_publication_endpoints.iter() {
        if let Err(e) = HTTP_CLIENT.post(endpoint).json(&swc_payload).send().await {
            tracing::error!("Failed to publish coupon message: {}", e);
        }
    }
}
