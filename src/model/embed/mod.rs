use serde::{Deserialize, Serialize};

pub const TORAHIKO_COLOR: u32 = 0xdfd16c;

#[derive(Deserialize, Serialize)]
pub struct EmbedObject {
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub color: Option<u32>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedImage>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub video: Option<EmbedVideo>,
    pub provider: Option<EmbedProvider>,
    pub author: Option<EmbedAuthor>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedImage {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedThumbnail {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedVideo {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedProvider {
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct EmbedAuthor {
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[allow(dead_code)]
impl EmbedObject {
    pub fn new() -> Self {
        EmbedObject {
            title: None,
            kind: None,
            description: None,
            url: None,
            color: None,
            footer: None,
            image: None,
            thumbnail: None,
            video: None,
            provider: None,
            author: None,
            fields: None,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn footer(
        mut self,
        text: &str,
        icon_url: Option<String>,
        proxy_icon_url: Option<String>,
    ) -> Self {
        self.footer = Some(EmbedFooter {
            text: text.into(),
            icon_url,
            proxy_icon_url,
        });
        self
    }

    pub fn image(
        mut self,
        url: &str,
        proxy_url: Option<String>,
        height: Option<u32>,
        width: Option<u32>,
    ) -> Self {
        self.image = Some(EmbedImage {
            url: Some(url.into()),
            proxy_url,
            height,
            width,
        });
        self
    }

    pub fn thumbnail(
        mut self,
        url: &str,
        proxy_url: Option<String>,
        height: Option<u32>,
        width: Option<u32>,
    ) -> Self {
        self.thumbnail = Some(EmbedThumbnail {
            url: Some(url.into()),
            proxy_url,
            height,
            width,
        });
        self
    }

    pub fn video(
        mut self,
        url: &str,
        proxy_url: Option<String>,
        height: Option<u32>,
        width: Option<u32>,
    ) -> Self {
        self.video = Some(EmbedVideo {
            url: Some(url.into()),
            proxy_url,
            height,
            width,
        });
        self
    }

    pub fn author(
        mut self,
        name: &str,
        icon_url: Option<String>,
        url: Option<String>,
        proxy_icon_url: Option<String>,
    ) -> Self {
        self.author = Some(EmbedAuthor {
            name: Some(name.into()),
            url,
            icon_url,
            proxy_icon_url,
        });
        self
    }

    pub fn field(mut self, name: &str, value: &str, inline: bool) -> Self {
        if let Some(ref mut fields) = self.fields {
            fields.push(EmbedField {
                name: name.into(),
                value: value.into(),
                inline: Some(inline),
            });
        } else {
            self.fields = Some(vec![EmbedField {
                name: name.into(),
                value: value.into(),
                inline: Some(inline),
            }]);
        }
        self
    }
}
