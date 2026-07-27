use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiration: i64,
    pub jwt_refresh_expiration: i64,
    pub rate_limit_anonymous: u32,
    pub rate_limit_authenticated: u32,
    pub max_note_size: usize,
    pub max_notes_per_user: i64,
    pub max_collaborators_per_note: usize,
    pub max_ws_connections: usize,
    pub cors_allowed_origins: Vec<String>,
    pub brevo_api_key: String,
    pub app_url: String,
    pub email_from: String,
    pub imagekit_private_key: String,
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub vapid_subject: String,
    pub self_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Parse CORS origins from comma-separated string
        let cors_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")?,
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
            redis_url: env::var("REDIS_URL")?,
            jwt_secret: env::var("JWT_SECRET")?,
            jwt_access_expiration: env::var("JWT_ACCESS_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .unwrap_or(86400),
            jwt_refresh_expiration: env::var("JWT_REFRESH_EXPIRATION")
                .unwrap_or_else(|_| "604800".to_string())
                .parse()
                .unwrap_or(604800),
            rate_limit_anonymous: env::var("RATE_LIMIT_ANONYMOUS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
            rate_limit_authenticated: env::var("RATE_LIMIT_AUTHENTICATED")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            max_note_size: env::var("MAX_NOTE_SIZE")
                .unwrap_or_else(|_| "102400".to_string())
                .parse()
                .unwrap_or(102400),
            max_notes_per_user: env::var("MAX_NOTES_PER_USER")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            max_collaborators_per_note: env::var("MAX_COLLABORATORS_PER_NOTE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            max_ws_connections: env::var("MAX_WS_CONNECTIONS")
                .unwrap_or_else(|_| "2000".to_string())
                .parse()
                .unwrap_or(2000),
            cors_allowed_origins: cors_origins,
            brevo_api_key: env::var("BREVO_API_KEY").unwrap_or_default(),
            app_url: env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            email_from: env::var("EMAIL_FROM").unwrap_or_else(|_| "noreply@noteflow.app".to_string()),
            imagekit_private_key: env::var("IMAGEKIT_PRIVATE_KEY").unwrap_or_default(),
            vapid_public_key: env::var("VAPID_PUBLIC_KEY").unwrap_or_default(),
            vapid_private_key: env::var("VAPID_PRIVATE_KEY").unwrap_or_default(),
            vapid_subject: env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:notifications@noteflow.app".to_string()),
            self_url: env::var("SELF_URL")
                .unwrap_or_else(|_| format!("http://{}:{}", 
                    env::var("HOST").unwrap_or_else(|_| "localhost".to_string()), 
                    env::var("PORT").unwrap_or_else(|_| "8080".to_string()))),
        })
    }
}
