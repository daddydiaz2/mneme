use std::path::PathBuf;

/// Clave pública para encriptación.
#[derive(Debug, Clone)]
pub enum RecipientKey {
    /// SSH key (ed25519 o RSA).
    Ssh(String),
    /// age native key (age1...).
    Age(String),
}

impl RecipientKey {
    /// Carga desde archivo de clave pública SSH.
    pub fn from_ssh_file(path: &PathBuf) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(crate::error::MnemeError::Io)?;
        Ok(RecipientKey::Ssh(content.trim().to_string()))
    }

    /// Carga desde string (detecta tipo automáticamente).
    pub fn from_string(s: &str) -> crate::error::Result<Self> {
        let s = s.trim();
        if s.starts_with("age1") {
            Ok(RecipientKey::Age(s.to_string()))
        } else {
            Ok(RecipientKey::Ssh(s.to_string()))
        }
    }

    /// Retorna el tipo como string para almacenar en DB.
    pub fn key_type(&self) -> &str {
        match self {
            RecipientKey::Ssh(s) => {
                if s.contains("ssh-ed25519") {
                    "ssh-ed25519"
                } else if s.contains("ssh-rsa") {
                    "ssh-rsa"
                } else {
                    "ssh"
                }
            }
            RecipientKey::Age(_) => "age",
        }
    }

    /// Retorna la representación en string para almacenar en DB.
    pub fn public_key_string(&self) -> String {
        match self {
            RecipientKey::Ssh(s) | RecipientKey::Age(s) => s.clone(),
        }
    }
}

/// Identidad para desencriptación (clave privada).
#[derive(Debug)]
pub enum IdentityKey {
    /// SSH private key.
    Ssh(PathBuf),
    /// age native identity file.
    Age(PathBuf),
}

impl IdentityKey {
    /// Detecta y carga la identidad disponible en el sistema.
    pub fn detect() -> crate::error::Result<Self> {
        // 1. MNEME_IDENTITY env var
        if let Ok(val) = std::env::var("MNEME_IDENTITY") {
            return Self::from_path(&PathBuf::from(val));
        }
        // 2. ~/.ssh/id_ed25519
        if let Some(mut home) = dirs::home_dir() {
            home.push(".ssh");
            let ed25519 = home.join("id_ed25519");
            if ed25519.exists() {
                return Ok(IdentityKey::Ssh(ed25519));
            }
            // 3. ~/.ssh/id_rsa
            let rsa = home.join("id_rsa");
            if rsa.exists() {
                return Ok(IdentityKey::Ssh(rsa));
            }
        }
        // 4. ~/.age/key.txt
        if let Some(mut home) = dirs::home_dir() {
            home.push(".age");
            let key = home.join("key.txt");
            if key.exists() {
                return Ok(IdentityKey::Age(key));
            }
        }
        Err(crate::error::MnemeError::IdentityNotLoaded)
    }

    /// Carga desde path explícito.
    pub fn from_path(path: &PathBuf) -> crate::error::Result<Self> {
        if !path.exists() {
            return Err(crate::error::MnemeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("identity file not found: {}", path.display()),
            )));
        }
        // Detectar tipo por extensión o contenido
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            return Ok(IdentityKey::Age(path.clone()));
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if filename.starts_with("id_") {
            Ok(IdentityKey::Ssh(path.clone()))
        } else {
            // Intentar leer primeras líneas para detectar
            let content = std::fs::read_to_string(path)?;
            if content.contains("AGE-SECRET-KEY") {
                Ok(IdentityKey::Age(path.clone()))
            } else {
                Ok(IdentityKey::Ssh(path.clone()))
            }
        }
    }

    /// Retorna el path de la identidad.
    pub fn path(&self) -> &PathBuf {
        match self {
            IdentityKey::Ssh(p) | IdentityKey::Age(p) => p,
        }
    }
}
