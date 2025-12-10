// Helper: generate Apple Music Developer Token (JWT) using ES256 (P-256) and a .p8 private key
// This is a template. For production, store the private key in a secret manager.

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    iat: i64,
    exp: i64,
}

/// Load a .p8 private key (PKCS#8 PEM) and sign a JWT for Apple Music.
/// - `team_id`: Apple Developer Team ID
/// - `key_id`: The Music Key ID (Key identifier shown in App Store Connect)
/// - `private_key_pem_path`: path to the downloaded .p8 file
/// - `ttl_seconds`: desired token lifetime in seconds (<= 6 months recommended)
pub fn generate_developer_token(team_id: &str, key_id: &str, private_key_pem_path: &str, ttl_seconds: i64) -> Result<String> {
    // read private key
    let pem = fs::read_to_string(private_key_pem_path).context("failed to read private key file")?;

    // jsonwebtoken's EncodingKey::from_ec_pem expects the PKCS8 PEM for EC keys
    let encoding_key = EncodingKey::from_ec_pem(pem.as_bytes()).context("invalid EC PEM")?;

    let now = Utc::now();
    let iat = now.timestamp();
    let exp = (now + Duration::seconds(ttl_seconds)).timestamp();

    let claims = Claims { iss: team_id, iat, exp };

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id.to_owned());

    let token = jsonwebtoken::encode(&header, &claims, &encoding_key).context("failed to encode JWT")?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gen_token_template_compiles() {
        // This test ensures the function compiles. Do not run in CI (no key available).
        // We simply ensure the function exists and returns an error when key missing.
        let res = generate_developer_token("TEAMID", "KEYID", "nonexistent.p8", 300);
        assert!(res.is_err());
    }
}

