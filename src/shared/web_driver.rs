use crate::model::dialog_info::DialogInfo;
use once_cell::sync::OnceCell;
use thirtyfour::prelude::*;
use thirtyfour::{ChromeCapabilities, OptionRect};

static CAPABILITIES: OnceCell<ChromeCapabilities> = OnceCell::new();
static WEB_DRIVER: OnceCell<WebDriver> = OnceCell::new();

const WEB_DRIVER_ADDRESS: &str = "http://localhost:65535";
const DIALOG_TEMPLATE_FILE_NAME: &str = "/asset/dialog/template.html";

const DIALOG_SCRIPT: &str = r#"
            document.getElementById('text').innerText = `{text}`;
            document.getElementById('background').src = './images/backgrounds/{background}.png';
            document.getElementById('ribbon').src = './images/ribbons/{character}.png';
            document.getElementById('character').src = './images/characters/{character}.png';
        "#;

pub async fn get_dialog(dialog_info: DialogInfo) -> anyhow::Result<Vec<u8>> {
    initialize().await?;
    if let Some(driver) = WEB_DRIVER.get() {
        driver
            .get(String::from(WEB_DRIVER_ADDRESS) + DIALOG_TEMPLATE_FILE_NAME)
            .await?;

        let sanitized_text = dialog_info
            .text
            .replace("$", "")
            .replace("{", "")
            .replace("}", "")
            .replace("`", "");

        let script = DIALOG_SCRIPT
            .replace("{text}", &sanitized_text)
            .replace("{background}", &dialog_info.background)
            .replace("{character}", &dialog_info.character);

        driver.execute_script(&script).await?;
        let screenshot = driver.screenshot_as_png().await?;
        Ok(screenshot)
    } else {
        Ok(vec![])
    }
}

async fn initialize() -> anyhow::Result<()> {
    let capabilities = CAPABILITIES.get_or_init(|| {
        let mut caps = DesiredCapabilities::chrome();
        caps.set_headless()
            .expect("Failed to set capability to headless.");
        caps.add_chrome_arg("--no-sandbox")
            .expect("Failed to add Chrome argument --no-sandbox.");
        caps.add_chrome_arg("--disable-dev-shm-usage")
            .expect("Failed to add Chrome argument --disable-dev-shm-usage.");
        caps
    });

    if WEB_DRIVER.get().is_none() {
        let driver = WebDriver::new(WEB_DRIVER_ADDRESS, capabilities.clone()).await?;
        driver
            .set_window_rect(OptionRect::new().with_size(810, 1080))
            .await?;
        WEB_DRIVER
            .set(driver)
            .expect("Failed to set web driver's OnceCell.");
    }
    Ok(())
}
