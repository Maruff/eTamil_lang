// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// Backend milestone 4: Authentication & Authorization Module
// JWT-based auth with role-based access control (RBAC)

use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const JWT_SECRET_ENV: &str = "ETAMIL_JWT_SECRET";
const BCRYPT_COST: u32 = 12;

/// The signing secret, read from the environment.
///
/// A hardcoded constant would let anyone holding the binary forge a token for
/// any role, so when the variable is unset we fall back to a random secret
/// that lives only as long as this process: tokens stop working after a
/// restart, which is noisy but safe.
///
/// Resolved once per process. It used to be generated afresh on every call,
/// which was invisible only because a single `AuthManager` held both keys
/// from one call — any second caller signed with a different random secret,
/// so tokens issued through one path could never be verified through another.
fn jwt_secret() -> &'static [u8] {
    static SECRET: OnceLock<Vec<u8>> = OnceLock::new();

    SECRET
        .get_or_init(|| match std::env::var(JWT_SECRET_ENV) {
            Ok(secret) if !secret.is_empty() => secret.into_bytes(),
            _ => {
                eprintln!(
                    "⚠️  {} is not set — using a random per-process secret. \
                     Issued tokens will not survive a restart. Set {} in production.",
                    JWT_SECRET_ENV, JWT_SECRET_ENV
                );
                format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).into_bytes()
            }
        })
        .as_slice()
}

/// JWT Claims structure for access tokens
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub roles: Vec<String>, // RBAC roles
    pub iat: i64,           // Issued at
    pub exp: i64,           // Expiration
}

/// User credentials for login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response with tokens
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// User record in system
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<String>,
}

/// Authentication manager
pub struct AuthManager {
    users: HashMap<String, User>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new() -> Self {
        let secret = jwt_secret();
        Self {
            users: HashMap::new(),
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
        }
    }

    /// Register a new user
    pub fn register_user(
        &mut self,
        email: &str,
        password: &str,
        roles: Vec<String>,
    ) -> Result<User, String> {
        if self.users.values().any(|u| u.email == email) {
            return Err("User already exists".to_string());
        }

        let password_hash =
            hash(password, BCRYPT_COST).map_err(|_| "Failed to hash password".to_string())?;

        let user = User {
            id: format!("user_{}", uuid::Uuid::new_v4()),
            email: email.to_string(),
            password_hash,
            roles,
        };

        self.users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    /// Login user and generate JWT tokens
    pub fn login(&self, email: &str, password: &str) -> Result<AuthResponse, String> {
        let user = self
            .users
            .values()
            .find(|u| u.email == email)
            .ok_or("User not found".to_string())?;

        // `verify` returns Ok(false) for a wrong password and only errors on
        // a malformed hash. Discarding that bool — keeping just the Err —
        // meant every password was accepted for any account that existed.
        // The test suite missed it because its failure case used an unknown
        // user, which is caught by the lookup above.
        let password_matches =
            verify(password, &user.password_hash).map_err(|_| "Invalid credentials".to_string())?;

        if !password_matches {
            return Err("Invalid credentials".to_string());
        }

        let now = Utc::now();
        let access_exp = now + Duration::hours(1);
        let refresh_exp = now + Duration::days(7);

        let access_claims = TokenClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
        };

        let refresh_claims = TokenClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            roles: vec!["refresh".to_string()],
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|_| "Failed to generate access token".to_string())?;

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|_| "Failed to generate refresh token".to_string())?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            expires_in: 3600,
        })
    }

    /// Verify and decode JWT token
    pub fn verify_token(&self, token: &str) -> Result<TokenClaims, String> {
        decode::<TokenClaims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|_| "Invalid token".to_string())
    }

    /// Check if user has required role
    pub fn has_role(&self, claims: &TokenClaims, required_role: &str) -> bool {
        claims.roles.contains(&required_role.to_string())
    }

    /// Get user by ID
    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.users.get(user_id).cloned()
    }
}

// --- The primitives eTamil calls ------------------------------------------
//
// Only what the language genuinely cannot express: bcrypt, HMAC-SHA256,
// base64 and a source of randomness. Everything above them stays in eTamil —
// who a user is, which routes need which role, where the accounts live.
//
// A token's payload crosses this boundary as **JSON text**, read and written
// on the other side by nUlakam/vativam/jEcAZ.qmz. That keeps the host from having to
// know what a claim means, and it means no Value-to-serde conversion lives
// here at all.

/// Hash a password for storage.
pub fn hash_password(password: &str) -> Result<String, String> {
    hash(password, BCRYPT_COST)
        .map_err(|_| "கடவுச்சொல்லை மறைக்க முடியவில்லை  (cannot hash the password)".to_string())
}

/// Check a password against a stored hash.
///
/// Returns whether it matched; a malformed stored hash is the error case.
/// Note the shape — an earlier caller threw away exactly this bool and so
/// accepted every password.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, String> {
    verify(password, password_hash)
        .map_err(|_| "சேமித்த மறையீடு செல்லாதது  (the stored password hash is not valid)".to_string())
}

/// Sign a JSON payload into a token that expires after `ttl_seconds`.
///
/// `iat` and `exp` are set here rather than taken from the caller: an expiry
/// a handler could choose is an expiry an attacker could choose.
pub fn issue_token(payload_json: &str, ttl_seconds: i64) -> Result<String, String> {
    let mut claims: serde_json::Value = serde_json::from_str(payload_json).map_err(|_| {
        "குறியீட்டுச் சுமை செல்லாத ஜேசான்  (the token payload is not valid JSON)".to_string()
    })?;

    let object = claims.as_object_mut().ok_or_else(|| {
        "குறியீட்டுச் சுமை ஒரு பொருளாக இருக்க வேண்டும்  (the token payload must be a record)".to_string()
    })?;

    let now = Utc::now().timestamp();
    object.insert("iat".to_string(), serde_json::json!(now));
    object.insert("exp".to_string(), serde_json::json!(now + ttl_seconds));

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret()),
    )
    .map_err(|_| "குறியீட்டை உருவாக்க முடியவில்லை  (cannot issue the token)".to_string())
}

/// Verify a token's signature and expiry, yielding its claims as JSON text.
pub fn read_token(token: &str) -> Result<String, String> {
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp"]);
    // jsonwebtoken allows 60 seconds of clock skew by default, which silently
    // kept accepting tokens for a minute after they expired. Stated here so
    // the tolerance is a decision rather than an inherited default; five
    // seconds still covers ordinary skew between a client and this server.
    validation.leeway = 5;

    let data =
        decode::<serde_json::Value>(token, &DecodingKey::from_secret(jwt_secret()), &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    "குறியீடு காலாவதியானது  (the token has expired)".to_string()
                }
                _ => "குறியீடு செல்லாதது  (the token is not valid)".to_string(),
            })?;

    serde_json::to_string(&data.claims).map_err(|_| {
        "குறியீட்டின் உள்ளடக்கத்தைப் படிக்க முடியவில்லை  (cannot read the token's claims)".to_string()
    })
}

/// Which key signed a token, read from its header without verifying anything.
///
/// A provider like Entra ID publishes many keys and rotates them, so the only
/// way to know which one to check against is to look at the token's own `kid`
/// first. Nothing here is trusted: the header is read, the signature is not,
/// and the answer is only useful for choosing a key to then verify with.
pub fn token_header(token: &str) -> Result<(String, String), String> {
    let header = jsonwebtoken::decode_header(token).map_err(|_| {
        "குறியீட்டின் தலைப்பைப் படிக்க முடியவில்லை  (cannot read the token header)".to_string()
    })?;

    Ok((header.kid.unwrap_or_default(), format!("{:?}", header.alg)))
}

/// Verify an RS256 token against a public key given as its JWK components.
///
/// `n` and `e` are the base64url modulus and exponent straight out of a JWKS
/// document. Fetching that document, choosing the key and caching it are the
/// language's business — this only answers whether a token is signed by the
/// key it was handed, and is for whom it claims to be.
///
/// The issuer and the audience are required, not optional. A token signed by
/// a real provider for a different application is a valid token; accepting it
/// because the audience went unchecked is how one tenant's login becomes
/// another application's session.
pub fn verify_rsa_token(
    token: &str,
    modulus: &str,
    exponent: &str,
    issuer: &str,
    audience: &str,
) -> Result<String, String> {
    if issuer.is_empty() || audience.is_empty() {
        return Err(
            "வழங்குநரும் பார்வையாளரும் தேவை  (the issuer and the audience are both required)".to_string(),
        );
    }

    let key = DecodingKey::from_rsa_components(modulus, exponent)
        .map_err(|_| "பொது சாவி செல்லாதது  (the public key is not valid)".to_string())?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_required_spec_claims(&["exp"]);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[audience]);
    // The same five seconds read_token allows, and for the same reason: a
    // stated tolerance rather than jsonwebtoken's inherited sixty.
    validation.leeway = 5;

    let data =
        decode::<serde_json::Value>(token, &key, &validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                "குறியீடு காலாவதியானது  (the token has expired)".to_string()
            }
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                "வழங்குநர் பொருந்தவில்லை  (the issuer does not match)".to_string()
            }
            jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                "பார்வையாளர் பொருந்தவில்லை  (the audience does not match)".to_string()
            }
            _ => "குறியீடு செல்லாதது  (the token is not valid)".to_string(),
        })?;

    serde_json::to_string(&data.claims).map_err(|_| {
        "குறியீட்டின் உள்ளடக்கத்தைப் படிக்க முடியவில்லை  (cannot read the token's claims)".to_string()
    })
}

/// RBAC middleware guard
pub struct RoleGuard {
    required_roles: Vec<String>,
}

impl RoleGuard {
    pub fn new(roles: Vec<&str>) -> Self {
        Self {
            required_roles: roles.iter().map(|r| r.to_string()).collect(),
        }
    }

    pub fn check(&self, claims: &TokenClaims) -> bool {
        self.required_roles
            .iter()
            .any(|role| claims.roles.contains(role))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_user() {
        let mut auth = AuthManager::new();
        let user = auth.register_user("user@example.com", "password123", vec!["user".to_string()]);
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.email, "user@example.com");
        assert!(user.roles.contains(&"user".to_string()));
    }

    #[test]
    fn test_login_success() {
        let mut auth = AuthManager::new();
        auth.register_user("user@example.com", "password123", vec!["admin".to_string()])
            .unwrap();
        let result = auth.login("user@example.com", "password123");
        assert!(result.is_ok());
        let auth_resp = result.unwrap();
        assert!(!auth_resp.access_token.is_empty());
        assert!(!auth_resp.refresh_token.is_empty());
    }

    #[test]
    fn test_login_failure() {
        let auth = AuthManager::new();
        let result = auth.login("unknown@example.com", "password");
        assert!(result.is_err());
    }

    // Regression: `verify`'s bool was discarded, so this login succeeded and
    // handed out a valid token for an account the caller had no password to.
    // The existing failure test only covered an unknown user, which the
    // lookup rejects before any password is checked.
    #[test]
    fn login_with_the_wrong_password_is_refused() {
        let mut auth = AuthManager::new();
        auth.register_user(
            "user@example.com",
            "correct-horse",
            vec!["user".to_string()],
        )
        .unwrap();

        assert!(auth.login("user@example.com", "wrong-password").is_err());
        assert!(auth.login("user@example.com", "").is_err());
        assert!(auth.login("user@example.com", "correct-horse").is_ok());
    }

    #[test]
    fn test_verify_token() {
        let mut auth = AuthManager::new();
        auth.register_user("user@example.com", "password123", vec!["user".to_string()])
            .unwrap();
        let auth_resp = auth.login("user@example.com", "password123").unwrap();
        let claims = auth.verify_token(&auth_resp.access_token);
        assert!(claims.is_ok());
        let claims = claims.unwrap();
        assert_eq!(claims.email, "user@example.com");
    }

    #[test]
    fn test_role_guard() {
        let guard = RoleGuard::new(vec!["admin"]);
        let claims = TokenClaims {
            sub: "user1".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["admin".to_string()],
            iat: 1000,
            exp: 2000,
        };
        assert!(guard.check(&claims));

        let claims_no_role = TokenClaims {
            sub: "user2".to_string(),
            email: "test2@example.com".to_string(),
            roles: vec!["user".to_string()],
            iat: 1000,
            exp: 2000,
        };
        assert!(!guard.check(&claims_no_role));
    }
}
