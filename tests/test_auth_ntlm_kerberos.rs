use quicpulse::middleware::auth::{parse_type2_message, NtlmAuth};
use reqwest::header::HeaderMap;

#[test]
fn test_ntlm_username_domain_parsing() {
    let ntlm1 = NtlmAuth::new("CORP\\alice", "secret");
    assert_eq!(ntlm1.username(), "alice");
    assert_eq!(ntlm1.domain(), Some("CORP"));

    let ntlm2 = NtlmAuth::new("bob@corp.local", "secret");
    assert_eq!(ntlm2.username(), "bob");
    assert_eq!(ntlm2.domain(), Some("corp.local"));

    let ntlm3 = NtlmAuth::new("charlie", "secret");
    assert_eq!(ntlm3.username(), "charlie");
    assert_eq!(ntlm3.domain(), None);

    let ntlm4 = NtlmAuth::with_domain("david", "secret", "MYDOMAIN").workstation("MYPC");
    assert_eq!(ntlm4.username(), "david");
    assert_eq!(ntlm4.domain(), Some("MYDOMAIN"));
}

#[test]
fn test_ntlm_type1_message_generation() {
    let ntlm = NtlmAuth::new("CORP\\alice", "Password123");
    let mut headers = HeaderMap::new();
    ntlm.apply(&mut headers).unwrap();

    let auth_header = headers.get("Authorization").unwrap().to_str().unwrap();
    assert!(auth_header.starts_with("NTLM "));
    let b64_part = auth_header.strip_prefix("NTLM ").unwrap();
    let decoded =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64_part).unwrap();
    assert_eq!(&decoded[0..8], b"NTLMSSP\0");
    assert_eq!(
        u32::from_le_bytes([decoded[8], decoded[9], decoded[10], decoded[11]]),
        1
    );
}

#[test]
fn test_ntlm_type2_parsing_and_type3_generation() {
    // Construct a minimal valid Type 2 Challenge message
    let mut type2_bytes = Vec::new();
    type2_bytes.extend_from_slice(b"NTLMSSP\0");
    type2_bytes.extend_from_slice(&2u32.to_le_bytes()); // Type 2
    type2_bytes.extend_from_slice(&0u16.to_le_bytes()); // target name len
    type2_bytes.extend_from_slice(&0u16.to_le_bytes()); // target name max len
    type2_bytes.extend_from_slice(&32u32.to_le_bytes()); // target name offset
    type2_bytes.extend_from_slice(&0x00080201u32.to_le_bytes()); // Flags (NTLM_NEGOTIATE_UNICODE | NTLM_NEGOTIATE_NTLM | EXTENDED_SESSIONSECURITY)
    type2_bytes.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]); // Server challenge nonce (8 bytes)

    let type2 = parse_type2_message(&type2_bytes).unwrap();
    assert_eq!(
        type2.server_challenge,
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );

    let ntlm = NtlmAuth::with_domain("alice", "Password123", "CORP");
    let type3_bytes = ntlm.process_challenge(&type2).unwrap();
    assert_eq!(&type3_bytes[0..8], b"NTLMSSP\0");
    assert_eq!(
        u32::from_le_bytes([
            type3_bytes[8],
            type3_bytes[9],
            type3_bytes[10],
            type3_bytes[11]
        ]),
        3
    );

    let type3_header = ntlm.generate_type3_header(&type2).unwrap();
    assert!(type3_header.to_str().unwrap().starts_with("NTLM "));
}
