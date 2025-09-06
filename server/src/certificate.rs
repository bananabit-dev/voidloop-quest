use bevy::prelude::*;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

/// Certificate digest generator for server TLS certificates
pub struct CertificateDigest;

impl CertificateDigest {
    /// Generate the certificate digest during server startup
    /// This will try multiple methods to get/generate the digest:
    /// 1. Read from LIGHTYEAR_CERTIFICATE_DIGEST environment variable
    /// 2. Generate from certificate file if available
    /// 3. Generate a runtime digest based on server identity
    pub fn generate() -> Option<String> {
        // First try to read from environment variable (existing behavior)
        if let Ok(digest) = env::var("LIGHTYEAR_CERTIFICATE_DIGEST") {
            if !digest.is_empty() {
                info!("🔐 Using certificate digest from environment variable");
                return Some(digest);
            }
        }

        // Try to generate from certificate file
        if let Some(digest) = Self::generate_from_cert_file() {
            info!("🔐 Generated certificate digest from certificate file");
            return Some(digest);
        }

        // Generate a runtime digest based on server identity
        if let Some(digest) = Self::generate_runtime_digest() {
            info!("🔐 Generated runtime certificate digest for development");
            return Some(digest);
        }

        warn!("🔐 Could not generate certificate digest - this may affect WebTransport clients");
        None
    }

    /// Try to generate digest from certificate file
    /// Looks for common certificate file paths
    fn generate_from_cert_file() -> Option<String> {
        let cert_paths = [
            "certs/server.crt",
            "certs/game-server.crt", 
            "/etc/ssl/certs/server.crt",
        ];

        // Check static paths first
        for &path_str in cert_paths.iter() {
            let path = Path::new(path_str);
            if path.exists() {
                match Self::compute_cert_digest(path) {
                    Ok(digest) => {
                        info!("📋 Computed certificate digest from: {}", path_str);
                        return Some(digest);
                    }
                    Err(e) => {
                        warn!("❌ Failed to compute digest from {}: {}", path_str, e);
                    }
                }
            }
        }

        // Check environment variable path
        if let Ok(env_path) = env::var("SERVER_CERT_PATH") {
            let path = Path::new(&env_path);
            if path.exists() {
                match Self::compute_cert_digest(path) {
                    Ok(digest) => {
                        info!("📋 Computed certificate digest from env path: {}", env_path);
                        return Some(digest);
                    }
                    Err(e) => {
                        warn!("❌ Failed to compute digest from env path {}: {}", env_path, e);
                    }
                }
            }
        }

        None
    }

    /// Compute SHA-256 digest of a certificate file
    fn compute_cert_digest(cert_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let cert_data = fs::read(cert_path)?;
        
        // If it's a PEM file, extract the certificate content
        let cert_der = if cert_data.starts_with(b"-----BEGIN CERTIFICATE-----") {
            Self::pem_to_der(&cert_data)?
        } else {
            cert_data
        };

        let mut hasher = Sha256::new();
        hasher.update(&cert_der);
        let digest = hasher.finalize();
        
        Ok(hex::encode(digest))
    }

    /// Convert PEM certificate to DER format for hashing
    fn pem_to_der(pem_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let pem_str = std::str::from_utf8(pem_data)?;
        let lines: Vec<&str> = pem_str.lines().collect();
        
        let mut in_cert = false;
        let mut cert_lines = Vec::new();
        
        for line in lines {
            if line == "-----BEGIN CERTIFICATE-----" {
                in_cert = true;
                continue;
            }
            if line == "-----END CERTIFICATE-----" {
                break;
            }
            if in_cert {
                cert_lines.push(line);
            }
        }
        
        let cert_b64 = cert_lines.join("");
        let cert_der = base64_decode(&cert_b64)?;
        Ok(cert_der)
    }

    /// Generate a runtime digest based on server identity
    /// This is useful for development and when no certificate file is available
    fn generate_runtime_digest() -> Option<String> {
        // Create a deterministic digest based on server properties
        let mut hasher = Sha256::new();
        
        // Add server identification components
        if let Ok(hostname) = env::var("HOSTNAME") {
            hasher.update(hostname.as_bytes());
        }
        
        if let Ok(fqdn) = env::var("SERVER_FQDN") {
            hasher.update(fqdn.as_bytes());
        }
        
        // Add build information for uniqueness
        hasher.update(env!("VERGEN_GIT_SHA").as_bytes());
        hasher.update(env!("VERGEN_BUILD_TIMESTAMP").as_bytes());
        
        // Add a static component to ensure we have some content
        hasher.update(b"voidloop-quest-server-development-digest");
        
        let digest = hasher.finalize();
        Some(hex::encode(digest))
    }
}

/// Simple base64 decoder for PEM certificates
fn base64_decode(input: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    let chars: Vec<char> = input.chars().filter(|c| !c.is_whitespace()).collect();
    
    // Simple base64 alphabet
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for &ch in chars.iter() {
        if ch == '=' {
            break;
        }
        
        if let Some(index) = alphabet.find(ch) {
            buffer = (buffer << 6) | (index as u32);
            bits += 6;
            
            if bits >= 8 {
                result.push(((buffer >> (bits - 8)) & 0xFF) as u8);
                bits -= 8;
            }
        }
    }
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_digest_generation() {
        let digest = CertificateDigest::generate_runtime_digest();
        assert!(digest.is_some());
        
        let digest_str = digest.unwrap();
        assert_eq!(digest_str.len(), 64); // SHA-256 hex is 64 characters
        assert!(digest_str.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_base64_decode() {
        let input = "SGVsbG8gV29ybGQ="; // "Hello World" in base64
        let result = base64_decode(input).unwrap();
        assert_eq!(result, b"Hello World");
    }
}