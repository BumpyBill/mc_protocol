use rsa::{PublicKeyParts, RsaPrivateKey, RsaPublicKey};

pub struct Server {
    pub public_key: RsaPublicKey,
    pub private_key: RsaPrivateKey,
    pub encoded_public_key: Vec<u8>,
    pub http_client: reqwest::Client,
}

impl Server {
    pub fn new() -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();

        let private_key = RsaPrivateKey::new(&mut rng, 1024).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);
        let encoded_public_key = rsa_der::public_key_to_der(
            &public_key.n().to_bytes_be(),
            &public_key.e().to_bytes_be(),
        );

        let http_client = reqwest::Client::new();

        Ok(Self {
            private_key,
            public_key,
            encoded_public_key,
            http_client,
        })
    }
}
