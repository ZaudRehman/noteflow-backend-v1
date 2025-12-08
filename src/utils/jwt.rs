use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::utils::errors::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub email: String,
    pub exp: i64,     // expiration timestamp
    pub iat: i64,     // issued at timestamp
    pub token_type: TokenType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

pub struct JwtManager {
    secret: String,
    access_expiration: i64,
    refresh_expiration: i64,
}

impl JwtManager {
    pub fn new(secret: String, access_exp: i64, refresh_exp: i64) -> Self {
        Self {
            secret,
            access_expiration: access_exp,
            refresh_expiration: refresh_exp,
        }
    }

    pub fn generate_access_token(&self, user_id: Uuid, email: String) -> Result<String> {
        self.generate_token(user_id, email, TokenType::Access, self.access_expiration)
    }

    pub fn generate_refresh_token(&self, user_id: Uuid, email: String) -> Result<String> {
        self.generate_token(user_id, email, TokenType::Refresh, self.refresh_expiration)
    }

    fn generate_token(
        &self,
        user_id: Uuid,
        email: String,
        token_type: TokenType,
        expiration: i64,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = (now + Duration::seconds(expiration)).timestamp();

        let claims = Claims {
            sub: user_id.to_string(),
            email,
            exp,
            iat: now.timestamp(),
            token_type,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(format!("Token generation failed: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| AppError::AuthenticationError(format!("Invalid token: {}", e)))
    }

    pub fn verify_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;
        if claims.token_type != TokenType::Access {
            return Err(AppError::AuthenticationError(
                "Invalid token type".to_string(),
            ));
        }
        Ok(claims)
    }

    pub fn verify_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;
        if claims.token_type != TokenType::Refresh {
            return Err(AppError::AuthenticationError(
                "Invalid token type".to_string(),
            ));
        }
        Ok(claims)
    }
}