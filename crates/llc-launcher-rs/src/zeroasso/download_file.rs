use crate::zeroasso::client;
use eyre::Context;
use futures::AsyncReadExt;
use llc_rs::LLCConfig;
use nyquest::Request;
use sha2::Digest;
use std::sync::Arc;

const DEFAULT_BUFFER_SIZE: usize = 2 * 1024 * 1024; // 1 MiB

pub async fn run(
    llc_config: Arc<LLCConfig>,
    file_name: String,
    hash: Option<[u8; 32]>,
) -> eyre::Result<Vec<u8>> {
    let client = client()
        .await
        .inspect_err(|e| error!("Failed to create API client: {e}"))
        .context("无法创建 API 客户端。")?;
    let download_url = llc_config.download_url_for(&file_name);
    let req = Request::get(download_url.to_string());
    info!("Downloading file '{file_name}' from '{download_url}'");
    let res = client.request(req).await?;
    let buffer_size = res
        .content_length()
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_BUFFER_SIZE);

    let mut buffer = Vec::with_capacity(buffer_size);
    res.into_async_read().read_to_end(&mut buffer).await?;

    if let Some(expected_hash) = hash {
        let hash = sha2::Sha256::digest(buffer.as_slice());
        if hash.as_slice() != expected_hash {
            error!(
                "Hash mismatch for '{}': expected {}, got {}",
                file_name,
                hex::encode(expected_hash),
                hex::encode(hash)
            );
            return Err(eyre::eyre!(
                "下载的 '{file_name}' 损坏，哈希不匹配，很可能是网络问题导致的，请稍后重试。",
            ));
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use llc_rs::LLCConfig;
    use std::{fs, io::Cursor};

    #[tokio::test]
    async fn test_download_file() {
        let dirs = directories::BaseDirs::new().unwrap();

        let llc_config = Arc::new(LLCConfig::default());
        let mut hash = [0u8; 32];
        hex::decode_to_slice(
            "c33a20843375ac465e5fa010539f59212a90f75634463e79becdc35dfc93ce6f",
            &mut hash,
        )
        .unwrap();

        let result = run(
            llc_config,
            "LimbusLocalize_2025070503.7z".to_string(),
            Some(hash),
        )
        .await
        .unwrap();
        let reader = Cursor::new(result);
        let dir = dirs
            .cache_dir()
            .join("llc-launcher-rs")
            .join("test_download_file");
        sevenz_rust::decompress(reader, &dir).unwrap();
        println!("Downloaded and extracted to: {}", dir.display());

        let ver_file = dir
            .join("LimbusCompany_Data")
            .join("Lang")
            .join("LLC_zh-CN")
            .join("Info")
            .join("version.json");
        assert!(
            ver_file.exists(),
            "Version file does not exist: {}",
            ver_file.display()
        );
        let ver_parsed: serde_json::Value =
            serde_json::from_reader(fs::File::open(ver_file).unwrap())
                .expect("Failed to read version file");
        println!("Version file content: {:?}", ver_parsed);
        assert_eq!(
            ver_parsed["version"].as_u64().unwrap(),
            2025070503,
            "Version mismatch in downloaded file"
        );
        println!("Test passed: File downloaded and verified successfully.");
        fs::remove_dir_all(dir).expect("Failed to clean up test directory");
    }
}
