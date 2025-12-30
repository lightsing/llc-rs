use crate::utils::ResultExt;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLCConfig {
    #[serde(default = "default_npm_registries", rename = "npm-registries")]
    npm_registries: Vec<Url>,
}

impl LLCConfig {
    /// Get the NPM registries.
    pub fn npm_registries(&self) -> &[Url] {
        &self.npm_registries
    }
}

impl Default for LLCConfig {
    fn default() -> Self {
        LLCConfig {
            npm_registries: default_npm_registries(),
        }
    }
}

pub(crate) fn default_npm_registries() -> Vec<Url> {
    vec![
        Url::parse("https://registry.npmmirror.com").infallible(),
        Url::parse("https://registry.npmjs.org").infallible(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    static CURRENT_DEFAULT: &str = r#"npm-registries = [
    "https://registry.npmmirror.com/",
    "https://registry.npmjs.org/",
]
"#;
    #[test]
    fn test_config() {
        let config: LLCConfig =
            toml::from_str(CURRENT_DEFAULT).expect("Failed to deserialize config");

        let serialized = toml::to_string_pretty(&config).expect("Failed to serialize config");
        println!("{}", serialized);
        assert_eq!(serialized, CURRENT_DEFAULT);
    }

    #[test]
    fn test_cross_version_compatibility() {
        let old_toml = r#"[settings]
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
        let config: LLCConfig = toml::from_str(old_toml).expect("Failed to deserialize old config");

        let serialized = toml::to_string_pretty(&config).expect("Failed to serialize config");
        println!("{}", serialized);

        assert_eq!(serialized, CURRENT_DEFAULT);
    }
}
