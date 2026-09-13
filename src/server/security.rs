use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use serde::{Deserialize, Serialize};

const IDENTIFIER_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const PASSWORD_ALPHABET: [char; 52] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionClaims {
    pub sub: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| anyhow!("failed to hash password"))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|_| anyhow!("stored password hash is invalid"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_token() -> String {
    nanoid::nanoid!(12, &PASSWORD_ALPHABET)
}

pub fn generate_identifier() -> String {
    nanoid::nanoid!(10, &IDENTIFIER_ALPHABET)
}

pub fn generate_initial_password() -> String {
    nanoid::nanoid!(16, &PASSWORD_ALPHABET)
}

pub fn create_session_token(claims: &SessionClaims) -> Result<String> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|_| anyhow!("failed to create session token"))
}

pub fn verify_session_token(token: &str) -> Result<SessionClaims> {
    decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret()?.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| anyhow!("invalid or expired session token"))
}

fn jwt_secret() -> Result<String> {
    let secret =
        std::env::var("JWT_SECRET").map_err(|_| anyhow!("JWT_SECRET must be configured"))?;
    if secret.len() < 32 {
        return Err(anyhow!("JWT_SECRET must be at least 32 characters"));
    }
    Ok(secret)
}

pub fn query_value<'a>(parameters: &'a str, name: &str) -> Option<&'a str> {
    parameters
        .trim_start_matches('?')
        .split(&['&', '?'])
        .filter_map(|parameter| parameter.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{generate_identifier, hash_password, query_value, verify_password};
    #[test]
    fn password_hash_verifies_only_the_original_password() {
        let password_hash = hash_password("correct horse battery staple").unwrap();

        assert!(verify_password("correct horse battery staple", &password_hash).unwrap());
        assert!(!verify_password("incorrect", &password_hash).unwrap());
    }

    #[test]
    fn generated_identifier_uses_the_short_stream_alphabet() {
        let identifier = generate_identifier();
        let second_identifier = generate_identifier();

        assert_eq!(identifier.len(), 10);
        assert!(identifier
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()));
        assert_ne!(identifier, second_identifier);
    }

    #[test]
    fn query_value_finds_a_named_query_parameter() {
        assert_eq!(
            query_value("?token=secret&vhost=default", "token"),
            Some("secret")
        );
        assert_eq!(query_value("token=secret", "missing"), None);
    }
}
