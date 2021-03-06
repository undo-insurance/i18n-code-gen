use anyhow::Result;
use reqwest::{Client, Url};
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug)]
pub struct LokaliseClient {
    api_token: String,
    client: Client,
}

impl LokaliseClient {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            client: Client::new(),
        }
    }

    fn lokalise_url(&self, path: &str) -> Result<Url> {
        Ok(Url::parse(&format!(
            "https://api.lokalise.com/api2/{}",
            path
        ))?)
    }

    pub async fn projects(&self) -> Result<Vec<Project>> {
        #[derive(Deserialize)]
        struct Projects {
            projects: Vec<Project>,
        }
        let resp = self.req::<Projects>(self.lokalise_url("projects")?).await?;
        Ok(resp.projects)
    }

    pub async fn keys(&self, project: &Project) -> Result<Vec<Key>> {
        #[derive(Deserialize)]
        struct Keys {
            keys: Vec<Key>,
        }

        let per_page = 5000;
        let mut page = 1;

        let mut keys = vec![];

        loop {
            let mut url = self.lokalise_url(&format!("projects/{}/keys", project.project_id))?;
            url.query_pairs_mut()
                .append_pair("include_translations", "1");
            url.query_pairs_mut().append_pair("page", &page.to_string());
            url.query_pairs_mut()
                .append_pair("limit", &per_page.to_string());

            let resp = self.req::<Keys>(url).await?;

            page += 1;
            let keys_len = resp.keys.len();
            keys.extend(resp.keys);

            if keys_len < per_page {
                break;
            }
        }

        Ok(keys)
    }

    async fn req<T>(&self, url: Url) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let json = self
            .client
            .get(url.clone())
            .header("x-api-token", &self.api_token)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        match serde_json::from_value(json.clone()) {
            Ok(out) => Ok(out),
            Err(err) => {
                eprintln!("Failed to decode response from Lokalise");
                eprintln!("URL = {}", url);
                eprintln!(
                    "Response = {}",
                    serde_json::to_string_pretty(&json).unwrap()
                );
                eprintln!("API token = {}", self.api_token);
                eprintln!("-------------");

                Err(err.into())
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub project_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Key {
    pub key_id: i32,
    pub key_name: KeyName,
    pub translations: Vec<Translation>,
    pub is_plural: bool,
}

#[derive(Debug, Deserialize)]
pub struct Translation {
    pub language_iso: String,
    pub translation: String,
}

#[derive(Debug, Deserialize)]
pub struct KeyName {
    pub ios: String,
    pub android: String,
    pub web: String,
    pub other: String,
}

impl KeyName {
    pub fn all_same(&self) -> bool {
        self.ios == self.android && self.android == self.web && self.web == self.other
    }
}
