/*use jsonwebtoken as jwt;
use ring::signature::KeyPair;

pub fn parse_keypair<B: AsRef<[u8]>>(
    pem_data: B,
) -> Result<(jwt::EncodingKey, jwt::DecodingKey<'static>), anyhow::Error> {
    // TODO: validate it is a prime256v1 PKCS#8 PEM encoded private key

    let content = pem::parse(pem_data)?;
    let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        &content.contents,
    )?;
    Ok((
        jwt::EncodingKey::from_ec_der(&content.contents),
        jwt::DecodingKey::from_ec_der(keypair.public_key().as_ref()).into_static(),
    ))
} */
use jsonwebtoken as jwt;
use ring::signature::KeyPair;
use rustls::internal::msgs::enums::NamedGroup;
use rustls::internal::pemfile::{pkcs8_private_keys, CertType};

pub fn parse_keypair<B: AsRef<[u8]>> (
    pem_data: B,
) -> Result<(jwt::EncodingKey, jwt::DecodingKey), anyhow::Error> {
    // Validate that the provided PEM-encoded private key is a prime256v1 PKCS#8 key
    if !validate_prime256v1_pkcs8_pem(&pem_data) {
        return Err(anyhow::anyhow!("Invalid prime256v1 PKCS#8 PEM-encoded private key"));
    }

    let content = pem::parse(pem_data)?;
    let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        &content.contents,
    )?;
    Ok((
        jwt::EncodingKey::from_ec_der(&content.contents),
        jwt::DecodingKey::from_ec_der(keypair.public_key().as_ref()).into(),
    ))
}

fn validate_prime256v1_pkcs8_pem<B: AsRef<[u8]>>(pem_data: B) -> bool {
    // Extract PKCS#8 private key from PEM-encoded data
    let mut pkcs8_keys = pkcs8_private_keys(pem_data.as_ref());
    if pkcs8_keys.len() != 1 {
        return false;
    }
    let pkcs8_key = &pkcs8_keys[0];

    // Extract algorithm identifier from PKCS#8 PrivateKeyInfo structure
    let algorithm_identifier = match pkcs8_key.cert_type {
        CertType::RSA => return false,
        CertType::EC => pkcs8_key
            .parsed_ec_private_key
            .as_ref()
            .unwrap()
            .algorithm
            .as_ref()
            .unwrap(),
    };

    // Compare algorithm identifier with prime256v1 OID
    match algorithm_identifier.named_group {
        Some(NamedGroup::X25519) => true,
        _ => false,
    }
}

