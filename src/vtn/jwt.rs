use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuthRole {
    BL,
    VEN,
}

pub struct BLUser(pub Claims);
pub struct VENUser(pub Claims);
pub struct User(pub Claims);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    exp: usize,
    nbf: usize,
    sub: String,
    role: AuthRole,
    ven: Option<String>,
}

impl JwtManager {
    /// Create a new JWT manager from a base64 encoded secret
    pub fn from_base64_secret(secret: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let encoding_key = EncodingKey::from_base64_secret(secret)?;
        let decoding_key = DecodingKey::from_base64_secret(secret)?;
        Ok(Self::new(encoding_key, decoding_key))
    }

    /// Create a new JWT manager from some secret bytes
    pub fn from_secret(secret: &[u8]) -> Self {
        let encoding_key = EncodingKey::from_secret(secret);
        let decoding_key = DecodingKey::from_secret(secret);
        Self::new(encoding_key, decoding_key)
    }

    /// Create a new JWT manager with a specific encoding and decoding key
    pub fn new(encoding_key: EncodingKey, decoding_key: DecodingKey) -> Self {
        Self {
            encoding_key,
            decoding_key,
        }
    }

    /// Create a new JWT token with the given claims and expiration time
    pub fn create(
        &self,
        expires_in: std::time::Duration,
        client_id: String,
        role: AuthRole,
        ven: Option<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now();
        let exp = now + expires_in;

        let claims = Claims {
            exp: exp.timestamp() as usize,
            nbf: now.timestamp() as usize,
            sub: client_id,
            role,
            ven,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(token)
    }

    /// Decode and validate a given JWT token, returning the validated claims
    pub fn decode_and_validate(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let validation = jsonwebtoken::Validation::default();
        let token_data = jsonwebtoken::decode::<Claims>(&token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}
