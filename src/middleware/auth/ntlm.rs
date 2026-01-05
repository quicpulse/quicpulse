//! NTLM Authentication (Windows Integrated Auth)
//!
//! Implements full NTLM (NT LAN Manager) challenge-response authentication.
//! This is commonly used for Windows domain authentication.
//!
//! The NTLM flow:
//! 1. Client sends Type 1 (Negotiate) message
//! 2. Server responds with Type 2 (Challenge) message containing a nonce
//! 3. Client sends Type 3 (Authenticate) message with encrypted response
//!
//! This implementation supports NTLMv2 which is the modern, secure variant.

use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use super::AuthError;

// NTLM Flag constants
const NTLM_NEGOTIATE_UNICODE: u32 = 0x00000001;
const NTLM_NEGOTIATE_OEM: u32 = 0x00000002;
const NTLM_REQUEST_TARGET: u32 = 0x00000004;
const NTLM_NEGOTIATE_NTLM: u32 = 0x00000200;
const NTLM_NEGOTIATE_ALWAYS_SIGN: u32 = 0x00008000;
const NTLM_NEGOTIATE_EXTENDED_SESSIONSECURITY: u32 = 0x00080000;
const NTLM_NEGOTIATE_TARGET_INFO: u32 = 0x00800000;
const NTLM_NEGOTIATE_128: u32 = 0x20000000;
const NTLM_NEGOTIATE_56: u32 = 0x80000000;

/// NTLM authentication credentials and state
#[derive(Debug, Clone)]
pub struct NtlmAuth {
    /// Username (can include domain as DOMAIN\user or user@domain)
    username: String,
    /// Password
    password: String,
    /// Optional domain (extracted from username or provided separately)
    domain: Option<String>,
    /// Optional workstation name
    workstation: Option<String>,
}

/// Parsed Type 2 (Challenge) message from server
#[derive(Debug)]
pub struct Type2Message {
    /// Server challenge nonce (8 bytes)
    pub server_challenge: [u8; 8],
    /// Negotiate flags from server
    pub flags: u32,
    /// Target name (domain/server name)
    pub target_name: Option<String>,
    /// Target info blob (AV_PAIRs for NTLMv2)
    pub target_info: Option<Vec<u8>>,
}

impl NtlmAuth {
    /// Create new NTLM auth with username and password
    /// Username can be in formats: "user", "DOMAIN\\user", or "user@domain"
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let username_str = username.into();
        let (user, domain) = Self::parse_username(&username_str);

        Self {
            username: user,
            password: password.into(),
            domain,
            workstation: None,
        }
    }

    /// Create NTLM auth with explicit domain
    pub fn with_domain(
        username: impl Into<String>,
        password: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            domain: Some(domain.into()),
            workstation: None,
        }
    }

    /// Set the workstation name
    pub fn workstation(mut self, name: impl Into<String>) -> Self {
        self.workstation = Some(name.into());
        self
    }

    /// Parse username to extract domain if present
    fn parse_username(username: &str) -> (String, Option<String>) {
        // DOMAIN\user format
        if let Some((domain, user)) = username.split_once('\\') {
            return (user.to_string(), Some(domain.to_string()));
        }
        // user@domain format
        if let Some((user, domain)) = username.split_once('@') {
            return (user.to_string(), Some(domain.to_string()));
        }
        // Just username
        (username.to_string(), None)
    }

    /// Apply NTLM authentication - sends Type 1 (Negotiate) message
    /// This is the initial message to start NTLM handshake
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        // Generate NTLM Type 1 (Negotiate) message
        let type1_msg = self.generate_type1_message();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&type1_msg);

        let header_value = format!("NTLM {}", encoded);
        let value = HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;

        headers.insert(AUTHORIZATION, value);
        Ok(())
    }

    /// Process Type 2 challenge and generate Type 3 response
    pub fn process_challenge(&self, type2: &Type2Message) -> Result<Vec<u8>, AuthError> {
        self.generate_type3_message(type2)
    }

    /// Generate Type 3 Authorization header from a Type 2 challenge
    pub fn generate_type3_header(&self, type2: &Type2Message) -> Result<HeaderValue, AuthError> {
        let type3_msg = self.generate_type3_message(type2)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&type3_msg);
        let header_value = format!("NTLM {}", encoded);
        HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))
    }

    /// Generate NTLM Type 1 (Negotiate) message
    fn generate_type1_message(&self) -> Vec<u8> {
        let mut msg = Vec::with_capacity(40);

        // Signature: "NTLMSSP\0"
        msg.extend_from_slice(b"NTLMSSP\0");

        // Type: 1 (Negotiate)
        msg.extend_from_slice(&1u32.to_le_bytes());

        // Flags
        let flags: u32 = NTLM_NEGOTIATE_UNICODE
            | NTLM_NEGOTIATE_OEM
            | NTLM_REQUEST_TARGET
            | NTLM_NEGOTIATE_NTLM
            | NTLM_NEGOTIATE_ALWAYS_SIGN
            | NTLM_NEGOTIATE_EXTENDED_SESSIONSECURITY
            | NTLM_NEGOTIATE_TARGET_INFO
            | NTLM_NEGOTIATE_128
            | NTLM_NEGOTIATE_56;
        msg.extend_from_slice(&flags.to_le_bytes());

        // Domain security buffer (length, max_length, offset) - empty for Type 1
        msg.extend_from_slice(&0u16.to_le_bytes()); // length
        msg.extend_from_slice(&0u16.to_le_bytes()); // max_length
        msg.extend_from_slice(&0u32.to_le_bytes()); // offset

        // Workstation security buffer - empty for Type 1
        msg.extend_from_slice(&0u16.to_le_bytes()); // length
        msg.extend_from_slice(&0u16.to_le_bytes()); // max_length
        msg.extend_from_slice(&0u32.to_le_bytes()); // offset

        msg
    }

    /// Generate NTLM Type 3 (Authenticate) message
    fn generate_type3_message(&self, type2: &Type2Message) -> Result<Vec<u8>, AuthError> {
        use rand::RngCore;

        let domain = self.domain.as_deref().unwrap_or("");
        let workstation = self.workstation.as_deref().unwrap_or("");

        // Generate 8-byte client challenge
        let mut client_challenge = [0u8; 8];
        rand::rng().fill_bytes(&mut client_challenge);

        // Compute NTLMv2 response
        let (nt_response, lm_response) = self.compute_ntlmv2_response(
            &type2.server_challenge,
            &client_challenge,
            type2.target_info.as_deref(),
        )?;

        // Encode strings as UTF-16LE
        let domain_bytes = Self::to_utf16le(domain);
        let username_bytes = Self::to_utf16le(&self.username);
        let workstation_bytes = Self::to_utf16le(workstation);

        // Calculate offsets (Type 3 header is 64 bytes minimum)
        let base_offset: u32 = 64;
        let lm_offset = base_offset;
        let nt_offset = lm_offset + lm_response.len() as u32;
        let domain_offset = nt_offset + nt_response.len() as u32;
        let username_offset = domain_offset + domain_bytes.len() as u32;
        let workstation_offset = username_offset + username_bytes.len() as u32;

        let mut msg = Vec::with_capacity(256);

        // Signature: "NTLMSSP\0"
        msg.extend_from_slice(b"NTLMSSP\0");

        // Type: 3 (Authenticate)
        msg.extend_from_slice(&3u32.to_le_bytes());

        // LM Response security buffer
        msg.extend_from_slice(&(lm_response.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(lm_response.len() as u16).to_le_bytes());
        msg.extend_from_slice(&lm_offset.to_le_bytes());

        // NT Response security buffer
        msg.extend_from_slice(&(nt_response.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(nt_response.len() as u16).to_le_bytes());
        msg.extend_from_slice(&nt_offset.to_le_bytes());

        // Domain security buffer
        msg.extend_from_slice(&(domain_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(domain_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&domain_offset.to_le_bytes());

        // Username security buffer
        msg.extend_from_slice(&(username_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(username_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&username_offset.to_le_bytes());

        // Workstation security buffer
        msg.extend_from_slice(&(workstation_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&(workstation_bytes.len() as u16).to_le_bytes());
        msg.extend_from_slice(&workstation_offset.to_le_bytes());

        // Encrypted random session key (empty for basic NTLM)
        msg.extend_from_slice(&0u16.to_le_bytes());
        msg.extend_from_slice(&0u16.to_le_bytes());
        msg.extend_from_slice(&(workstation_offset + workstation_bytes.len() as u32).to_le_bytes());

        // Flags (match server's flags where appropriate)
        msg.extend_from_slice(&type2.flags.to_le_bytes());

        // Append data in order
        msg.extend_from_slice(&lm_response);
        msg.extend_from_slice(&nt_response);
        msg.extend_from_slice(&domain_bytes);
        msg.extend_from_slice(&username_bytes);
        msg.extend_from_slice(&workstation_bytes);

        Ok(msg)
    }

    /// Compute NTLMv2 response
    fn compute_ntlmv2_response(
        &self,
        server_challenge: &[u8; 8],
        client_challenge: &[u8; 8],
        target_info: Option<&[u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), AuthError> {
        use hmac::{Hmac, Mac};
        use md4::{Md4, Digest as Md4Digest};
        use md5_digest::Md5;

        // Step 1: Compute NT hash = MD4(UTF16LE(password))
        let password_utf16 = Self::to_utf16le(&self.password);
        let mut md4 = Md4::new();
        md4.update(&password_utf16);
        let nt_hash = md4.finalize();

        // Step 2: Compute NTLMv2 hash = HMAC-MD5(NT hash, UPPERCASE(username) + domain)
        let domain = self.domain.as_deref().unwrap_or("");
        let user_domain = format!("{}{}", self.username.to_uppercase(), domain);
        let user_domain_utf16 = Self::to_utf16le(&user_domain);

        type HmacMd5 = Hmac<Md5>;
        let mut hmac = HmacMd5::new_from_slice(&nt_hash)
            .map_err(|e| AuthError::InvalidCredentials(format!("HMAC error: {}", e)))?;
        hmac.update(&user_domain_utf16);
        let ntlmv2_hash = hmac.finalize().into_bytes();

        // Step 3: Build NTLMv2 blob
        let timestamp = Self::get_filetime();
        let mut blob = Vec::with_capacity(64 + target_info.map(|t| t.len()).unwrap_or(0));

        blob.extend_from_slice(&[0x01, 0x01]); // Blob version
        blob.extend_from_slice(&[0x00, 0x00]); // Reserved
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Reserved
        blob.extend_from_slice(&timestamp); // 8 bytes timestamp
        blob.extend_from_slice(client_challenge); // 8 bytes client challenge
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Reserved

        if let Some(info) = target_info {
            blob.extend_from_slice(info);
        }
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Terminator

        // Step 4: Compute NT response = HMAC-MD5(NTLMv2 hash, server_challenge + blob)
        let mut hmac = HmacMd5::new_from_slice(&ntlmv2_hash)
            .map_err(|e| AuthError::InvalidCredentials(format!("HMAC error: {}", e)))?;
        hmac.update(server_challenge);
        hmac.update(&blob);
        let nt_proof = hmac.finalize().into_bytes();

        // NT response = NT proof + blob
        let mut nt_response = Vec::with_capacity(16 + blob.len());
        nt_response.extend_from_slice(&nt_proof);
        nt_response.extend_from_slice(&blob);

        // Step 5: Compute LMv2 response = HMAC-MD5(NTLMv2 hash, server_challenge + client_challenge)
        let mut hmac = HmacMd5::new_from_slice(&ntlmv2_hash)
            .map_err(|e| AuthError::InvalidCredentials(format!("HMAC error: {}", e)))?;
        hmac.update(server_challenge);
        hmac.update(client_challenge);
        let lm_proof = hmac.finalize().into_bytes();

        // LM response = LM proof + client challenge
        let mut lm_response = Vec::with_capacity(24);
        lm_response.extend_from_slice(&lm_proof);
        lm_response.extend_from_slice(client_challenge);

        Ok((nt_response, lm_response))
    }

    /// Convert string to UTF-16LE bytes
    fn to_utf16le(s: &str) -> Vec<u8> {
        s.encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect()
    }

    /// Get current time as Windows FILETIME (100ns intervals since 1601)
    fn get_filetime() -> [u8; 8] {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Windows FILETIME epoch is January 1, 1601
        // UNIX epoch is January 1, 1970
        // Difference is 11644473600 seconds (369 years of days)
        const EPOCH_DIFF: u64 = 116444736000000000;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let filetime = now.as_secs() * 10_000_000 + now.subsec_nanos() as u64 / 100 + EPOCH_DIFF;
        filetime.to_le_bytes()
    }

    /// Get the username
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Get the domain if set
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
}

/// Parse a Type 2 (Challenge) message from the server
pub fn parse_type2_message(data: &[u8]) -> Result<Type2Message, AuthError> {
    if data.len() < 32 {
        return Err(AuthError::InvalidChallenge("Type 2 message too short".to_string()));
    }

    // Verify signature
    if &data[0..8] != b"NTLMSSP\0" {
        return Err(AuthError::InvalidChallenge("Invalid NTLM signature".to_string()));
    }

    // Verify message type
    let msg_type = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    if msg_type != 2 {
        return Err(AuthError::InvalidChallenge(format!(
            "Expected Type 2 message, got Type {}",
            msg_type
        )));
    }

    // Parse flags
    let flags = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);

    // Extract server challenge (always at offset 24)
    let mut server_challenge = [0u8; 8];
    server_challenge.copy_from_slice(&data[24..32]);

    // Parse target name security buffer
    let target_name = if data.len() >= 20 {
        let len = u16::from_le_bytes([data[12], data[13]]) as usize;
        let offset = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        if len > 0 && offset + len <= data.len() {
            let target_bytes = &data[offset..offset + len];
            Some(String::from_utf16_lossy(
                &target_bytes.chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)]))
                    .collect::<Vec<_>>()
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Parse target info (if present, at offset 40)
    let target_info = if data.len() >= 48 && (flags & NTLM_NEGOTIATE_TARGET_INFO) != 0 {
        let len = u16::from_le_bytes([data[40], data[41]]) as usize;
        let offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;
        if len > 0 && offset + len <= data.len() {
            Some(data[offset..offset + len].to_vec())
        } else {
            None
        }
    } else {
        None
    };

    Ok(Type2Message {
        server_challenge,
        flags,
        target_name,
        target_info,
    })
}

/// Extract NTLM Type 2 message from WWW-Authenticate header
pub fn extract_type2_from_header(headers: &HeaderMap) -> Result<Type2Message, AuthError> {
    let auth_header = headers
        .get(WWW_AUTHENTICATE)
        .ok_or_else(|| AuthError::InvalidChallenge("No WWW-Authenticate header".to_string()))?
        .to_str()
        .map_err(|_| AuthError::InvalidChallenge("Invalid WWW-Authenticate header".to_string()))?;

    // Look for NTLM or Negotiate scheme
    let token = if let Some(rest) = auth_header.strip_prefix("NTLM ") {
        rest.trim()
    } else if let Some(rest) = auth_header.strip_prefix("Negotiate ") {
        rest.trim()
    } else {
        return Err(AuthError::InvalidChallenge(
            "WWW-Authenticate header is not NTLM or Negotiate".to_string()
        ));
    };

    // Decode base64
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(token)
        .map_err(|e| AuthError::InvalidChallenge(format!("Base64 decode error: {}", e)))?;

    parse_type2_message(&decoded)
}

/// Negotiate (SPNEGO) authentication - auto-selects Kerberos or NTLM
#[derive(Debug, Clone)]
pub struct NegotiateAuth {
    /// Inner auth - usually NTLM as fallback
    inner: NtlmAuth,
}

impl NegotiateAuth {
    /// Create Negotiate auth (will attempt Kerberos, fall back to NTLM)
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            inner: NtlmAuth::new(username, password),
        }
    }

    /// Apply Negotiate authentication header (Type 1)
    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), AuthError> {
        let type1_msg = self.inner.generate_type1_message();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&type1_msg);

        let header_value = format!("Negotiate {}", encoded);
        let value = HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))?;

        headers.insert(AUTHORIZATION, value);
        Ok(())
    }

    /// Process Type 2 challenge and generate Type 3 header
    pub fn generate_type3_header(&self, type2: &Type2Message) -> Result<HeaderValue, AuthError> {
        let type3_msg = self.inner.generate_type3_message(type2)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&type3_msg);
        let header_value = format!("Negotiate {}", encoded);
        HeaderValue::from_str(&header_value)
            .map_err(|e| AuthError::InvalidHeader(e.to_string()))
    }
}

/// Kerberos authentication
#[derive(Debug, Clone)]
pub struct KerberosAuth {
    /// Principal name (user@REALM)
    principal: String,
    /// Password (for kinit-style auth)
    #[allow(dead_code)]
    password: String,
}

impl KerberosAuth {
    /// Create Kerberos auth with principal and password
    pub fn new(principal: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            principal: principal.into(),
            password: password.into(),
        }
    }

    /// Apply Kerberos authentication header
    /// Note: Full Kerberos requires GSSAPI and is platform-dependent.
    pub fn apply(&self, _headers: &mut HeaderMap) -> Result<(), AuthError> {
        Err(AuthError::KerberosNotConfigured)
    }

    /// Get the principal
    pub fn principal(&self) -> &str {
        &self.principal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_domain_backslash() {
        let auth = NtlmAuth::new("DOMAIN\\user", "pass");
        assert_eq!(auth.username(), "user");
        assert_eq!(auth.domain(), Some("DOMAIN"));
    }

    #[test]
    fn test_parse_domain_at() {
        let auth = NtlmAuth::new("user@domain.com", "pass");
        assert_eq!(auth.username(), "user");
        assert_eq!(auth.domain(), Some("domain.com"));
    }

    #[test]
    fn test_parse_simple_username() {
        let auth = NtlmAuth::new("user", "pass");
        assert_eq!(auth.username(), "user");
        assert_eq!(auth.domain(), None);
    }

    #[test]
    fn test_ntlm_type1_message() {
        let auth = NtlmAuth::new("user", "pass");
        let mut headers = HeaderMap::new();
        auth.apply(&mut headers).unwrap();

        let value = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(value.starts_with("NTLM "));

        // Decode and verify signature
        let encoded = &value[5..];
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(&decoded[0..8], b"NTLMSSP\0");

        // Verify type 1
        let msg_type = u32::from_le_bytes([decoded[8], decoded[9], decoded[10], decoded[11]]);
        assert_eq!(msg_type, 1);
    }

    #[test]
    fn test_negotiate_auth() {
        let auth = NegotiateAuth::new("user", "pass");
        let mut headers = HeaderMap::new();
        auth.apply(&mut headers).unwrap();

        let value = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(value.starts_with("Negotiate "));
    }

    #[test]
    fn test_parse_type2_message() {
        // Minimal valid Type 2 message
        let mut type2 = vec![0u8; 56];
        type2[0..8].copy_from_slice(b"NTLMSSP\0"); // Signature
        type2[8..12].copy_from_slice(&2u32.to_le_bytes()); // Type 2
        // Target name buffer (empty)
        // Flags
        type2[20..24].copy_from_slice(&(NTLM_NEGOTIATE_UNICODE | NTLM_NEGOTIATE_NTLM).to_le_bytes());
        // Server challenge
        type2[24..32].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let parsed = parse_type2_message(&type2).unwrap();
        assert_eq!(parsed.server_challenge, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_utf16le_encoding() {
        let encoded = NtlmAuth::to_utf16le("test");
        assert_eq!(encoded, vec![0x74, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74, 0x00]);
    }

    #[test]
    fn test_type3_generation() {
        let auth = NtlmAuth::new("user", "password");

        let type2 = Type2Message {
            server_challenge: [1, 2, 3, 4, 5, 6, 7, 8],
            flags: NTLM_NEGOTIATE_UNICODE | NTLM_NEGOTIATE_NTLM,
            target_name: Some("SERVER".to_string()),
            target_info: None,
        };

        let type3 = auth.generate_type3_message(&type2).unwrap();

        // Verify signature
        assert_eq!(&type3[0..8], b"NTLMSSP\0");

        // Verify type 3
        let msg_type = u32::from_le_bytes([type3[8], type3[9], type3[10], type3[11]]);
        assert_eq!(msg_type, 3);
    }
}
