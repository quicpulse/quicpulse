use quicpulse::middleware::auth::{DigestAlgorithm, DigestAuth, DigestChallenge};

#[test]
fn test_digest_algorithm_parsing() {
    assert_eq!(DigestAlgorithm::from_str("MD5"), DigestAlgorithm::MD5);
    assert_eq!(
        DigestAlgorithm::from_str("sha-256"),
        DigestAlgorithm::SHA256
    );
    assert_eq!(DigestAlgorithm::from_str("SHA256"), DigestAlgorithm::SHA256);
    assert_eq!(
        DigestAlgorithm::from_str("sha-512-256"),
        DigestAlgorithm::SHA512_256
    );
    assert_eq!(DigestAlgorithm::from_str("unknown"), DigestAlgorithm::MD5);

    assert_eq!(DigestAlgorithm::MD5.as_str(), "MD5");
    assert_eq!(DigestAlgorithm::SHA256.as_str(), "SHA-256");
    assert_eq!(DigestAlgorithm::SHA512_256.as_str(), "SHA-512-256");
}

#[test]
fn test_digest_challenge_parsing() {
    let header = "Digest realm=\"testrealm@host.com\", qop=\"auth,auth-int\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\", algorithm=SHA-256, stale=false";
    let challenge = DigestChallenge::parse(header).unwrap();

    assert_eq!(challenge.realm, "testrealm@host.com");
    assert_eq!(challenge.nonce, "dcd98b7102dd2f0e8b11d0f600bfb0c093");
    assert_eq!(challenge.algorithm, DigestAlgorithm::SHA256);
    assert_eq!(challenge.qop, Some("auth,auth-int".to_string()));
    assert_eq!(
        challenge.opaque,
        Some("5ccc069c403ebaf9f0171e9517f40e41".to_string())
    );
    assert!(!challenge.stale);
}

#[test]
fn test_digest_challenge_missing_required_fields() {
    let missing_nonce = "Digest realm=\"myrealm\"";
    assert!(DigestChallenge::parse(missing_nonce).is_err());

    let missing_realm = "Digest nonce=\"abc123nonce\"";
    assert!(DigestChallenge::parse(missing_realm).is_err());
}

#[test]
fn test_digest_auth_response_generation() {
    let auth = DigestAuth::from_credentials("Mufasa:CircleOfLife").unwrap();
    let challenge = DigestChallenge {
        realm: "myrealm@host.com".to_string(),
        nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
        algorithm: DigestAlgorithm::MD5,
        qop: Some("auth".to_string()),
        opaque: Some("5ccc069c403ebaf9f0171e9517f40e41".to_string()),
        stale: false,
        domain: None,
    };

    let header_val = auth
        .respond_to_challenge(&challenge, "GET", "/dir/index.html")
        .unwrap();
    assert!(header_val.starts_with("Digest "));
    assert!(header_val.contains("username=\"Mufasa\""));
    assert!(header_val.contains("realm=\"myrealm@host.com\""));
    assert!(header_val.contains("nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\""));
    assert!(header_val.contains("uri=\"/dir/index.html\""));
    assert!(header_val.contains("response="));
    assert!(header_val.contains("qop=\"auth\"") || header_val.contains("qop=auth"));
}
