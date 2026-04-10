//! Authenticated client for writing records to a user's PDS via OAuth + DPoP.
//!
//! Uses the session's access token (DPoP-bound) and per-session DPoP key to
//! make authenticated requests to the user's PDS. Handles the DPoP nonce
//! retry flow mandated by ATProto.

use crate::auth::Session;
use crate::auth::dpop::DpopKey;

/// Errors from PDS client operations.
#[derive(Debug, thiserror::Error)]
pub enum PdsClientError {
    #[error("failed to build DPoP key from session: {0}")]
    DpopKey(String),

    #[error("failed to build DPoP proof: {0}")]
    DpopProof(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("PDS returned {status}: {body}")]
    PdsError { status: u16, body: String },

    #[error("failed to parse PDS response: {0}")]
    ParseResponse(String),
}

/// Result of creating a record on a PDS.
#[derive(Debug, Clone)]
pub struct CreateRecordResult {
    pub uri: String,
    pub cid: String,
}

/// Create a record on the authenticated user's PDS.
///
/// Uses `com.atproto.repo.createRecord` with DPoP + bearer token auth.
/// Retries once if the server demands a DPoP nonce on the first attempt.
pub async fn create_record(
    http: &reqwest::Client,
    session: &Session,
    collection: &str,
    rkey: Option<&str>,
    record: &serde_json::Value,
) -> Result<CreateRecordResult, PdsClientError> {
    let dpop_key = DpopKey::from_private_key_b64(
        &session.dpop_private_key_b64,
        format!("session-{}", session.did),
    )
    .map_err(|e| PdsClientError::DpopKey(e.to_string()))?;

    let url = format!(
        "{}/xrpc/com.atproto.repo.createRecord",
        session.pds_url.trim_end_matches('/')
    );

    let mut body = serde_json::json!({
        "repo": session.did,
        "collection": collection,
        "record": record,
    });
    if let Some(rk) = rkey {
        body["rkey"] = serde_json::Value::String(rk.to_string());
    }

    // First attempt: use current nonce (or None).
    let resp = send_dpop_post(
        http,
        &dpop_key,
        &url,
        &session.access_token,
        session.dpop_nonce.as_deref(),
        &body,
    )
    .await?;

    let status = resp.status();
    if status.is_success() {
        return parse_create_response(resp).await;
    }

    // Capture any new nonce the server provided.
    let new_nonce = resp
        .headers()
        .get("dpop-nonce")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // If the server returned 401 with use_dpop_nonce, retry once with the new nonce.
    if status.as_u16() == 401 || status.as_u16() == 400 {
        let body_text = resp.text().await.unwrap_or_default();
        if body_text.contains("use_dpop_nonce") || body_text.contains("DPoP") {
            if let Some(ref nonce) = new_nonce {
                let retry = send_dpop_post(
                    http,
                    &dpop_key,
                    &url,
                    &session.access_token,
                    Some(nonce.as_str()),
                    &body,
                )
                .await?;

                if retry.status().is_success() {
                    return parse_create_response(retry).await;
                }

                let retry_status = retry.status().as_u16();
                let retry_body = retry.text().await.unwrap_or_default();
                return Err(PdsClientError::PdsError {
                    status: retry_status,
                    body: retry_body,
                });
            }
        }
        return Err(PdsClientError::PdsError {
            status: status.as_u16(),
            body: body_text,
        });
    }

    let body_text = resp.text().await.unwrap_or_default();
    Err(PdsClientError::PdsError {
        status: status.as_u16(),
        body: body_text,
    })
}

async fn send_dpop_post(
    http: &reqwest::Client,
    dpop_key: &DpopKey,
    url: &str,
    access_token: &str,
    nonce: Option<&str>,
    body: &serde_json::Value,
) -> Result<reqwest::Response, PdsClientError> {
    let proof = dpop_key
        .create_resource_proof("POST", url, access_token, nonce)
        .map_err(|e| PdsClientError::DpopProof(e.to_string()))?;

    Ok(http
        .post(url)
        .header("Authorization", format!("DPoP {access_token}"))
        .header("DPoP", proof)
        .json(body)
        .send()
        .await?)
}

async fn parse_create_response(
    resp: reqwest::Response,
) -> Result<CreateRecordResult, PdsClientError> {
    #[derive(serde::Deserialize)]
    struct CreateRecordResponse {
        uri: String,
        cid: String,
    }

    let body: CreateRecordResponse = resp
        .json()
        .await
        .map_err(|e| PdsClientError::ParseResponse(e.to_string()))?;

    Ok(CreateRecordResult {
        uri: body.uri,
        cid: body.cid,
    })
}

/// Delete a record from the authenticated user's PDS.
pub async fn delete_record(
    http: &reqwest::Client,
    session: &Session,
    collection: &str,
    rkey: &str,
) -> Result<(), PdsClientError> {
    let dpop_key = DpopKey::from_private_key_b64(
        &session.dpop_private_key_b64,
        format!("session-{}", session.did),
    )
    .map_err(|e| PdsClientError::DpopKey(e.to_string()))?;

    let url = format!(
        "{}/xrpc/com.atproto.repo.deleteRecord",
        session.pds_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "repo": session.did,
        "collection": collection,
        "rkey": rkey,
    });

    let resp = send_dpop_post(
        http,
        &dpop_key,
        &url,
        &session.access_token,
        session.dpop_nonce.as_deref(),
        &body,
    )
    .await?;

    if resp.status().is_success() {
        return Ok(());
    }

    // Nonce retry
    let new_nonce = resp
        .headers()
        .get("dpop-nonce")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if (status.as_u16() == 401 || status.as_u16() == 400)
        && (body_text.contains("use_dpop_nonce") || body_text.contains("DPoP"))
    {
        if let Some(ref nonce) = new_nonce {
            let retry = send_dpop_post(http, &dpop_key, &url, &session.access_token, Some(nonce), &body).await?;
            if retry.status().is_success() {
                return Ok(());
            }
        }
    }

    Err(PdsClientError::PdsError {
        status: status.as_u16(),
        body: body_text,
    })
}
