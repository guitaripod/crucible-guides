use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// What a Crucible app downloads: the bundle's bytes and a signature over them and the header,
/// so neither an old bundle replayed nor a header edited passes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub format: u32,
    pub key_id: String,
    pub sequence: u64,
    pub not_after: String,
    pub payload: String,
    pub signature: String,
}

/// The exact bytes the signature covers. The app rebuilds these to verify.
pub fn message(
    format: u32,
    key_id: &str,
    sequence: u64,
    not_after: &str,
    payload: &[u8],
) -> Vec<u8> {
    let mut m =
        format!("crucible-guides\n{format}\n{key_id}\n{sequence}\n{not_after}\n").into_bytes();
    m.extend_from_slice(payload);
    m
}

pub fn keygen() -> (String, String) {
    let key = SigningKey::generate(&mut rand::rngs::OsRng);
    (
        STANDARD.encode(key.to_bytes()),
        STANDARD.encode(key.verifying_key().to_bytes()),
    )
}

pub fn signing_key(secret_base64: &str) -> Result<SigningKey> {
    let bytes = STANDARD
        .decode(secret_base64.trim())
        .context("the key file is not base64")?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("the key is not 32 bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

pub fn seal(
    key: &SigningKey,
    key_id: &str,
    sequence: u64,
    not_after: &str,
    payload: &[u8],
) -> Envelope {
    let format = crate::model::FORMAT;
    let signature = key.sign(&message(format, key_id, sequence, not_after, payload));
    Envelope {
        format,
        key_id: key_id.to_string(),
        sequence,
        not_after: not_after.to_string(),
        payload: STANDARD.encode(payload),
        signature: STANDARD.encode(signature.to_bytes()),
    }
}

/// The payload of an envelope signed by `public_base64`, or why it is refused.
pub fn open(envelope: &Envelope, public_base64: &str) -> Result<Vec<u8>> {
    let public: [u8; 32] = STANDARD
        .decode(public_base64.trim())?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad public key"))?;
    let key = VerifyingKey::from_bytes(&public)?;
    let payload = STANDARD.decode(&envelope.payload)?;
    let sig: [u8; 64] = STANDARD
        .decode(&envelope.signature)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad signature length"))?;
    let m = message(
        envelope.format,
        &envelope.key_id,
        envelope.sequence,
        &envelope.not_after,
        &payload,
    );
    if key.verify(&m, &Signature::from_bytes(&sig)).is_err() {
        bail!("the signature does not match");
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_open() {
        let (secret, public) = keygen();
        let key = signing_key(&secret).unwrap();
        let env = seal(&key, "k1", 7, "2027-03-01", b"{\"a\":1}");
        assert_eq!(open(&env, &public).unwrap(), b"{\"a\":1}");
        let mut tampered = env.clone();
        tampered.sequence = 8;
        assert!(open(&tampered, &public).is_err());
        let (_, other) = keygen();
        assert!(open(&env, &other).is_err());
    }
}
