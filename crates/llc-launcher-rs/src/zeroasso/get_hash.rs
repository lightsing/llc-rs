use crate::zeroasso::utils::request_zeroasso_api;
use llc_rs::LLCConfig;
use serde::Deserialize;
use serde_with::serde_as;
use std::sync::Arc;

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct LLCHash {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub font_hash: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub main_hash: [u8; 32],
}

#[instrument(skip(llc_config), level = "trace")]
#[inline]
pub async fn run(llc_config: Arc<LLCConfig>) -> eyre::Result<LLCHash> {
    request_zeroasso_api(&llc_config, "v2/hash/get_hash").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use llc_rs::LLCConfig;

    #[tokio::test]
    async fn test_get_hash() {
        let llc_config = Arc::new(LLCConfig::default());
        let hash = run(llc_config).await;
        assert!(hash.is_ok());
        println!("[test_get_hash] Hash: {:?}", hash.unwrap());
    }
}
