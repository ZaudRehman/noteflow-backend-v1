use crate::utils::errors::{AppError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes128Gcm, Nonce,
};
use hkdf::Hkdf;
use p256::{
    ecdh::EphemeralSecret,
    elliptic_curve::sec1::ToEncodedPoint,
    PublicKey,
};
use rand::rngs::OsRng;
use sha2::Sha256;

const WEB_PUSH_INFO: &[u8] = b"WebPush: info\0";
const PAD_LEN: usize = 0;

/// Encrypt a Web Push payload according to RFC 8291.
/// Returns the complete body to POST to the endpoint.
pub fn encrypt_push_payload(
    client_p256dh: &[u8],
    auth_secret: &[u8],
    plaintext: &[u8],
) -> Result<EncryptedPush> {
    // Parse client public key (uncompressed 65-byte SEC1 format)
    let client_pk = PublicKey::from_sec1_bytes(client_p256dh)
        .map_err(|e| AppError::InternalError(format!("Invalid client key: {}", e)))?;

    // Generate ephemeral server key pair
    let ephemeral_sk = EphemeralSecret::random(&mut OsRng);
    let ephemeral_pk = PublicKey::from(&ephemeral_sk);
    let ephemeral_pk_raw = ephemeral_pk.to_encoded_point(false).as_bytes().to_vec();

    // ECDH shared secret
    let shared_secret = ephemeral_sk.diffie_hellman(&client_pk);
    let shared_secret_bytes = shared_secret.raw_secret_bytes().as_slice().to_vec();

    // HKDF info = "WebPush: info\0" || client_public_key || server_public_key
    let client_pk_raw = client_pk.to_encoded_point(false).as_bytes().to_vec();
    let mut info = WEB_PUSH_INFO.to_vec();
    info.extend_from_slice(&client_pk_raw);
    info.extend_from_slice(&ephemeral_pk_raw);

    // PRK = HKDF-SHA256(salt=auth_secret, IKM=shared_secret)
    let hkdf = Hkdf::<Sha256>::new(Some(auth_secret), &shared_secret_bytes);

    let mut prk = [0u8; 32];
    hkdf.expand(&info, &mut prk)
        .map_err(|e| AppError::InternalError(format!("PRK derivation failed: {}", e)))?;

    // CEK = HKDF-SHA256(PRK, "Content-Encoding: aes128gcm\0")
    let cek_info = b"Content-Encoding: aes128gcm\0";
    let cek_hkdf = Hkdf::<Sha256>::new(Some(&prk), cek_info);
    let mut cek = [0u8; 16];
    cek_hkdf.expand(&[], &mut cek)
        .map_err(|e| AppError::InternalError(format!("CEK derivation failed: {}", e)))?;

    // Nonce = HKDF-SHA256(PRK, "Content-Encoding: nonce\0")
    let nonce_info = b"Content-Encoding: nonce\0";
    let nonce_hkdf = Hkdf::<Sha256>::new(Some(&prk), nonce_info);
    let mut nonce_raw = [0u8; 12];
    nonce_hkdf.expand(&[], &mut nonce_raw)
        .map_err(|e| AppError::InternalError(format!("Nonce derivation failed: {}", e)))?;

    // Build padded plaintext: padding_delimiter(0x02) || padding(PAD_LEN * 0x00) || plaintext
    let mut padded = vec![0x02u8];
    padded.extend(std::iter::repeat(0u8).take(PAD_LEN));
    padded.extend_from_slice(plaintext);

    // AES-128-GCM encrypt
    let cipher = Aes128Gcm::new_from_slice(&cek)
        .map_err(|e| AppError::InternalError(format!("Cipher init failed: {}", e)))?;
    let nonce = Nonce::from_slice(&nonce_raw);
    let encrypted = cipher
        .encrypt(nonce, Payload { msg: &padded, aad: &[] })
        .map_err(|e| AppError::InternalError(format!("Encryption failed: {}", e)))?;

    // Build RFC 8188 header block: salt(16) || record_size(4 big-endian) || key_length(1) || public_key
    let salt = auth_secret; // we use auth_secret as the salt in the header
    let record_size = 4096u32;

    let mut header = Vec::new();
    header.extend_from_slice(&salt[..16.min(salt.len())]);
    header.extend_from_slice(&record_size.to_be_bytes());
    header.push(ephemeral_pk_raw.len() as u8);
    header.extend_from_slice(&ephemeral_pk_raw);

    let mut body = header;
    body.extend_from_slice(&encrypted);

    Ok(EncryptedPush {
        body,
        public_key: ephemeral_pk_raw,
    })
}

pub struct EncryptedPush {
    pub body: Vec<u8>,
    pub public_key: Vec<u8>,
}
