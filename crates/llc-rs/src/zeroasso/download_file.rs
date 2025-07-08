use crate::{
    LLCConfig,
    utils::{ClientExt, ResultExt},
    zeroasso::{ZeroAssoApiError, get_client},
};
use sha2::Digest;
use std::sync::Arc;

#[instrument(skip(llc_config, hash))]
pub async fn run(
    llc_config: Arc<LLCConfig>,
    file_name: String,
    hash: Option<[u8; 32]>,
) -> Result<Vec<u8>, ZeroAssoApiError> {
    let client = get_client().await?;
    let download_url = llc_config.download_url_for(&file_name);
    info!("Downloading file '{file_name}' from '{download_url}'");
    let bytes = client
        .download(download_url)
        .await
        .inspect_err(|e| error!("error downloading file: {e}"))?;

    if let Some(expected_hash) = hash {
        let actual_hash = sha2::Sha256::digest(bytes.as_slice());
        if actual_hash.as_slice() != expected_hash {
            error!(
                "Hash mismatch for '{}': expected {}, got {}",
                file_name,
                hex::encode(expected_hash),
                hex::encode(actual_hash)
            );
            return Err(ZeroAssoApiError::HashMismatch {
                file_name,
                expected_hash,
                actual_hash: actual_hash.try_into().infallible(),
            });
        }
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Cursor};

    smol_macros::test! {
        async fn test_download_file() {
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
            let dir = std::env::temp_dir()
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
}
