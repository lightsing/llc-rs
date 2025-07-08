use minijinja::{Environment, Template, context};
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use std::collections::BTreeMap;
use url::Url;
use crate::utils::ResultExt;

#[derive(Debug, Clone)]
pub struct LLCConfig {
    settings: Settings,
    github: GitHub,
    download_nodes: Environment<'static>,
    api_nodes: BTreeMap<String, Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Settings {
    download_node: String,
    api_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GitHub {
    repo: String,
    owner: String,
    api: Url,
}

impl LLCConfig {
    /// Get the download URL for a file based on the current settings.
    pub fn download_url_for(&self, file_name: impl AsRef<str>) -> Url {
        let template = self
            .download_nodes
            .get_template(self.settings.download_node.as_str())
            .expect("should always have a valid download node");
        template
            .render(context!(file_name => file_name.as_ref()))
            .expect("should always render successfully")
            .parse::<Url>()
            .expect("should always parse to a valid URL")
    }

    /// Get the API nodes URL based on the current settings.
    pub fn api_nodes(&self) -> impl Iterator<Item = &Url> + ExactSizeIterator {
        self.api_nodes.values()
    }

    /// Get fallback (other) download urls
    pub fn fallback_download_nodes<'a>(
        &'a self,
        file_name: &'a str,
    ) -> impl IntoIterator<Item = Url> + 'a {
        self.download_nodes
            .templates()
            .filter(|(name, _)| *name != self.settings.download_node.as_str())
            .map(move |(_, template)| {
                template
                    .render(context!(file_name => file_name))
                    .expect("should always render successfully")
                    .parse::<Url>()
                    .expect("should always parse to a valid URL")
            })
    }

    /// Get the GitHub settings.
    pub fn github(&self) -> &GitHub {
        &self.github
    }

    fn validate_and_fix(&mut self) {
        if self
            .download_nodes
            .get_template(&self.settings.download_node)
            .is_err()
        {
            warn!(
                "download node '{}' not found, using first available node",
                self.settings.download_node
            );
            self.settings.download_node = self
                .download_nodes
                .templates()
                .next()
                .expect("empty download nodes")
                .0
                .into();
        }
        if !self.api_nodes.contains_key(&self.settings.api_node) {
            warn!(
                "API node '{}' not found, using first available node",
                self.settings.api_node
            );
            self.settings.api_node = self
                .api_nodes
                .keys()
                .next()
                .expect("empty api nodes")
                .clone();
        }
    }
}

impl Default for LLCConfig {
    fn default() -> Self {
        LLCConfig {
            settings: Settings {
                download_node: "自动选择节点".into(),
                api_node: "零协会官方 API".into(),
            },
            github: GitHub {
                repo: "LocalizeLimbusCompany".into(),
                owner: "LocalizeLimbusCompany".into(),
                api: Url::parse("https://api.github.com").expect("valid GitHub API URL"),
            },
            download_nodes: {
                let mut env = Environment::new();
                env.add_template(
                    "自动选择节点",
                    "https://api.zeroasso.top/v2/download/files?file_name={{ file_name }}",
                ).infallible();
                env.add_template(
                    "零协会镇江节点",
                    "https://download.zeroasso.top/files/{{ file_name }}",
                ).infallible();
                env.add_template(
                    "CloudFlare CDN(海外)",
                    "https://cdn-download.zeroasso.top/files/{{ file_name }}",
                ).infallible();
                env
            },
            api_nodes: BTreeMap::from_iter([
                (
                    "零协会官方 API".into(),
                    Url::parse("https://api.zeroasso.top").infallible(),
                ),
                (
                    "CloudFlare CDN API(海外)".into(),
                    Url::parse("https://cdn-api.zeroasso.top").infallible(),
                ),
            ]),
        }
    }
}

impl Serialize for LLCConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "kebab-case")]
        struct LLCConfigHelper<'a> {
            settings: &'a Settings,
            github: &'a GitHub,
            download_node: Vec<DownloadNodeHelper<'a>>,
            api_node: Vec<NodeHelper<'a>>,
        }

        struct DownloadNodeHelper<'a> {
            name: &'a str,
            endpoint: Template<'a, 'a>,
        }

        #[derive(Serialize)]
        struct NodeHelper<'a> {
            name: &'a str,
            endpoint: &'a Url,
        }

        impl Serialize for DownloadNodeHelper<'_> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut s = serializer.serialize_struct("DownloadNodesHelper", 2)?;
                s.serialize_field("name", self.name)?;
                s.serialize_field("endpoint", &self.endpoint.source())?;
                s.end()
            }
        }

        let mut download_node = self
            .download_nodes
            .templates()
            .map(|(name, template)| DownloadNodeHelper {
                name,
                endpoint: template,
            })
            .collect::<Vec<_>>();
        download_node.sort_by_key(|node| node.name);

        let helper = LLCConfigHelper {
            settings: &self.settings,
            github: &self.github,
            download_node,
            api_node: self
                .api_nodes
                .iter()
                .map(|(name, endpoint)| NodeHelper {
                    name: name.as_str(),
                    endpoint,
                })
                .collect(),
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LLCConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct LLCConfigHelper {
            settings: Settings,
            github: GitHub,
            download_node: Vec<DownloadNodeHelper>,
            api_node: Vec<NodeHelper>,
        }

        #[derive(Deserialize)]
        struct DownloadNodeHelper {
            name: String,
            endpoint: String,
        }

        #[derive(Deserialize)]
        struct NodeHelper {
            name: String,
            endpoint: Url,
        }

        let helper = LLCConfigHelper::deserialize(deserializer)?;

        let download_nodes = if helper.download_node.is_empty() {
            warn!("no downloaded nodes configured, using default nodes");
            LLCConfig::default().download_nodes
        } else {
            let mut env = Environment::new();
            for node in helper.download_node {
                if let Err(e) = env.add_template_owned(node.name, node.endpoint) {
                    warn!("invalid download node found, skipping: {}", e);
                }
            }
            env
        };

        let mut api_nodes: BTreeMap<String, Url> = helper
            .api_node
            .into_iter()
            .map(|node| (node.name, node.endpoint))
            .collect();

        if api_nodes.is_empty() {
            warn!("no API nodes configured, using default nodes");
            api_nodes = LLCConfig::default().api_nodes;
        }

        let mut config = LLCConfig {
            settings: helper.settings,
            github: helper.github,
            download_nodes,
            api_nodes,
        };
        config.validate_and_fix();

        Ok(config)
    }
}

impl GitHub {
    /// Get the GitHub API URL.
    pub fn api_url(&self) -> &Url {
        &self.api
    }

    /// Get the Owner
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get the Repository name.
    pub fn repo(&self) -> &str {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let toml = r#"[settings]
download-node = "自动选择节点"
api-node = "零协会官方 API"

[github]
repo = "LocalizeLimbusCompany"
owner = "LocalizeLimbusCompany"
api = "https://api.github.com/"

[[download-node]]
name = "CloudFlare CDN(海外)"
endpoint = "https://cdn-download.zeroasso.top/files/{{ file_name }}"

[[download-node]]
name = "自动选择节点"
endpoint = "https://api.zeroasso.top/v2/download/files?file_name={{ file_name }}"

[[download-node]]
name = "零协会镇江节点"
endpoint = "https://download.zeroasso.top/files/{{ file_name }}"

[[api-node]]
name = "CloudFlare CDN API(海外)"
endpoint = "https://cdn-api.zeroasso.top/"

[[api-node]]
name = "零协会官方 API"
endpoint = "https://api.zeroasso.top/"
"#;

        let config: LLCConfig = toml::from_str(toml).expect("Failed to deserialize config");
        assert_eq!(config.settings.download_node, "自动选择节点");
        assert_eq!(config.settings.api_node, "零协会官方 API");

        let serialized = toml::to_string_pretty(&config).expect("Failed to serialize config");
        println!("{}", serialized);
        assert_eq!(serialized, toml);
    }
}
