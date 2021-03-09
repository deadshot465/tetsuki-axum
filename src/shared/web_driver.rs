use once_cell::sync::OnceCell;
use thirtyfour::prelude::*;
use thirtyfour::{ChromeCapabilities, OptionRect};

static CAPABILITIES: OnceCell<ChromeCapabilities> = OnceCell::new();
static WEB_DRIVER: OnceCell<WebDriver> = OnceCell::new();

const WEB_DRIVER_ADDRESS: &str = "http://localhost:3000";

pub async fn get_dialog() -> anyhow::Result<Vec<u8>> {
    initialize().await?;
    if let Some(driver) = WEB_DRIVER.get() {
        driver
            .get("http://localhost:8080/asset/dialog/template.html")
            .await?;
        driver
            .execute_script(
                r#"
            document.getElementById('text').innerText = 'Hello World!';
            document.getElementById('background').src = './images/backgrounds/cave.png';
            document.getElementById('ribbon').src = './images/ribbons/yuuto.png';
            document.getElementById('character').src = './images/characters/yuuto.png';
        "#,
            )
            .await?;
        let screenshot = driver.screenshot_as_png().await?;
        Ok(screenshot)
    } else {
        Ok(vec![])
    }
}

async fn initialize() -> anyhow::Result<()> {
    let capabilities = CAPABILITIES.get_or_init(|| {
        let mut caps = DesiredCapabilities::chrome();
        caps.set_headless();
        caps
    });
    if WEB_DRIVER.get().is_none() {
        let driver = WebDriver::new(WEB_DRIVER_ADDRESS, capabilities.clone()).await?;
        driver
            .set_window_rect(OptionRect::new().with_size(810, 1080))
            .await?;
        WEB_DRIVER.set(driver);
    }
    Ok(())
}
