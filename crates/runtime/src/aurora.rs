use anyhow::{Context, Result, bail};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DispenserResponse {
    email: Option<String>,
    #[serde(
        alias = "auth",
        alias = "token",
        alias = "authToken",
        alias = "auth_token"
    )]
    auth_token: Option<String>,
}

pub fn fetch_public_dispenser_auth() -> Result<(String, String)> {
    let dispenser_url = "https://auroraoss.com/api/auth";
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("com.aurora.store-4.6.2-64"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let client = Client::builder().default_headers(headers).build()?;
    let response = client.get(dispenser_url).send()?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!("received 403, request got blocked");
    } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
    {
        bail!("received 429 or 503 :(");
    } else if !status.is_success() {
        bail!("received unexpected HTTP status: {}", status);
    }

    let payload: DispenserResponse = response
        .json()
        .context("failed to parse JSON response from dispenser")?;

    let email = payload
        .email
        .context("dispenser did not return email field")?;
    let token = payload
        .auth_token
        .context("dispenser did not return auth token field")?;

    if !token.starts_with("ya29.") {
        bail!("invalid auth token received: {}", token);
    }

    Ok((email, token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use gpapi::Gpapi;
    use is_ci::cached;
    use std::fs;
    use std::path::Path;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    static AUTH_CACHE: OnceLock<(String, String)> = OnceLock::new();

    fn get_test_auth() -> Result<(String, String)> {
        if let Some(auth) = AUTH_CACHE.get() {
            return Ok(auth.clone());
        }
        let auth = fetch_public_dispenser_auth()?;
        let _ = AUTH_CACHE.set(auth.clone());
        Ok(auth)
    }

    #[test]
    fn test_fetch_dispenser_auth() -> Result<()> {
        if cached() {
            eprintln!("skipping test_fetch_dispenser_auth in CI");
            return Ok(());
        }

        let (email, token) = get_test_auth()?;
        println!("fetched email: {}", email);
        println!("fetched token prefix: {}...", &token[..15.min(token.len())]);

        assert!(
            !email.is_empty(),
            "expected a non-empty email/account identifier"
        );
        assert!(
            token.starts_with("ya29."),
            "expected token to start with 'ya29.'"
        );
        Ok(())
    }

    #[test]
    fn test_download_apk() -> Result<()> {
        if cached() {
            eprintln!("skipping test_download_apk in CI");
            return Ok(());
        }

        use gpapi::error::ErrorKind as GpapiErrorKind;

        let package_name = "com.duckduckgo.mobile.android";
        let output_dir = Path::new("./test_downloads");

        let (email, auth_token) = get_test_auth()?;

        let rt = Runtime::new()?;
        rt.block_on(async {
            let mut api = Gpapi::new("px_9a", &email);
            api.set_auth_token(auth_token);

            match api.login().await {
                Err(e) if matches!(e.kind(), GpapiErrorKind::TermsOfService) => {
                    api.accept_tos()
                        .await
                        .map_err(|e| anyhow!("{e}"))
                        .context("failed to accept the TOS of gplay")?;
                }
                Err(e) => {
                    return Err(anyhow!("{e}")).context("failed to log in to gplay");
                }
                Ok(()) => {}
            }

            let details = api
                .details(package_name)
                .await
                .map_err(|e| anyhow!("{e}"))
                .context("failed to fetch app details")?;
            assert!(details.is_some(), "app details should not be None");

            fs::create_dir_all(output_dir)?;
            api.download(package_name, None, true, false, false, output_dir, None)
                .await
                .map_err(|e| anyhow!("{e}"))
                .context("failed to download APK")?;

            Ok::<(), anyhow::Error>(())
        })?;

        let found_apk = fs::read_dir(Path::join(output_dir, package_name))?
            .filter_map(|e| e.ok())
            .any(|entry| {
                entry.path().extension().and_then(|e| e.to_str()) == Some("apk")
                    && entry.metadata().map(|m| m.len() > 0).unwrap_or(false)
            });

        assert!(found_apk, "no APK files found in output directory");

        let _ = fs::remove_dir_all(output_dir);
        Ok(())
    }
}
