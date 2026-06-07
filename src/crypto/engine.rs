use std::io::{Read, Write};

/// Motor de encriptación/desencriptación.
#[derive(Debug)]
pub struct CryptoEngine {
    recipients: Vec<crate::crypto::keys::RecipientKey>,
    identity: Option<crate::crypto::keys::IdentityKey>,
}

impl CryptoEngine {
    /// Crea un nuevo motor con los recipients dados.
    pub fn new(recipients: Vec<crate::crypto::keys::RecipientKey>) -> Self {
        Self { recipients, identity: None }
    }

    /// Encripta bytes para todos los recipients. Retorna ciphertext como Vec<u8>.
    pub fn encrypt(&self, plaintext: &[u8]) -> crate::error::Result<Vec<u8>> {
        if self.recipients.is_empty() {
            return Err(crate::error::MnemeError::NoRecipientsConfigured);
        }

        let mut age_recipients: Vec<Box<dyn age::Recipient + Send>> = Vec::new();

        for key in &self.recipients {
            match key {
                crate::crypto::keys::RecipientKey::Age(s) => {
                    let recipient: age::x25519::Recipient = s.parse()
                        .map_err(|e: &str| crate::error::MnemeError::Config(format!("invalid age key: {}", e)))?;
                    age_recipients.push(Box::new(recipient));
                }
                crate::crypto::keys::RecipientKey::Ssh(s) => {
                    let recipient: age::ssh::Recipient = s.parse()
                        .map_err(|e: age::ssh::ParseRecipientKeyError| crate::error::MnemeError::Config(format!("invalid ssh key: {:?}", e)))?;
                    age_recipients.push(Box::new(recipient));
                }
            }
        }

        let encryptor = age::Encryptor::with_recipients(age_recipients)
            .ok_or(crate::error::MnemeError::NoRecipientsConfigured)?;

        let mut ciphertext = vec![];
        let mut writer = encryptor.wrap_output(&mut ciphertext)
            .map_err(|e| crate::error::MnemeError::Config(format!("wrap output error: {}", e)))?;
        writer.write_all(plaintext)?;
        writer.finish()
            .map_err(|e| crate::error::MnemeError::Config(format!("finish error: {}", e)))?;

        Ok(ciphertext)
    }

    /// Encripta un string, retorna ciphertext en hex.
    pub fn encrypt_str(&self, plaintext: &str) -> crate::error::Result<String> {
        let ciphertext = self.encrypt(plaintext.as_bytes())?;
        Ok(hex::encode(ciphertext))
    }

    /// Desencripta ciphertext hex.
    pub fn decrypt_str(&mut self, ciphertext_hex: &str) -> crate::error::Result<String> {
        let ciphertext = hex::decode(ciphertext_hex)
            .map_err(|e| crate::error::MnemeError::Config(format!("hex decode error: {}", e)))?;

        let identity = self.identity.as_ref().ok_or(crate::error::MnemeError::IdentityNotLoaded)?;

        let decrypted = match identity {
            crate::crypto::keys::IdentityKey::Ssh(path) => {
                let key_content = std::fs::read_to_string(path)?;
                let identity = age::ssh::Identity::from_buffer(
                    std::io::BufReader::new(key_content.as_bytes()),
                    Some(path.display().to_string()),
                ).map_err(|_e| crate::error::MnemeError::DecryptionFailed)?;

                let decryptor = match age::Decryptor::new(ciphertext.as_slice())
                    .map_err(|_| crate::error::MnemeError::DecryptionFailed)? {
                    age::Decryptor::Recipients(d) => d,
                    _ => return Err(crate::error::MnemeError::DecryptionFailed),
                };

                let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
                    .map_err(|_| crate::error::MnemeError::DecryptionFailed)?;
                let mut output = String::new();
                reader.read_to_string(&mut output)?;
                output
            }
            crate::crypto::keys::IdentityKey::Age(path) => {
                let key_content = std::fs::read_to_string(path)?;
                let identity: age::x25519::Identity = key_content.trim().parse()
                    .map_err(|_e: &str| crate::error::MnemeError::DecryptionFailed)?;

                let decryptor = match age::Decryptor::new(ciphertext.as_slice())
                    .map_err(|_| crate::error::MnemeError::DecryptionFailed)? {
                    age::Decryptor::Recipients(d) => d,
                    _ => return Err(crate::error::MnemeError::DecryptionFailed),
                };

                let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
                    .map_err(|_| crate::error::MnemeError::DecryptionFailed)?;
                let mut output = String::new();
                reader.read_to_string(&mut output)?;
                output
            }
        };

        Ok(decrypted)
    }

    /// Verifica si puede desencriptar.
    pub fn can_decrypt(&self) -> bool {
        self.identity.is_some()
    }

    /// Carga la identidad de desencriptación.
    pub fn load_identity(&mut self) -> crate::error::Result<()> {
        let identity = crate::crypto::keys::IdentityKey::detect()?;
        self.identity = Some(identity);
        Ok(())
    }

    /// Carga identidad desde path específico.
    pub fn load_identity_from_path(&mut self, path: &std::path::PathBuf) -> crate::error::Result<()> {
        let identity = crate::crypto::keys::IdentityKey::from_path(path)?;
        self.identity = Some(identity);
        Ok(())
    }

    /// Retorna true si hay al menos un recipient configurado.
    pub fn has_recipients(&self) -> bool {
        !self.recipients.is_empty()
    }

    /// Retorna el alias/descripción de la clave usada.
    pub fn encrypted_for_label(&self) -> String {
        self.recipients.iter()
            .map(|r| r.key_type().to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}
