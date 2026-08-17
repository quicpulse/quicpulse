//! Authentication handling
//!
//! Provides support for various authentication methods:
//! - Basic authentication
//! - Digest authentication
//! - Bearer token authentication
//! - AWS Signature Version 4
//! - Google Cloud Platform (gcloud CLI)
//! - Azure CLI
//! - OAuth 2.0 Client Credentials
//! - OAuth 2.0 Authorization Code (with PKCE)
//! - OAuth 2.0 Device Flow

pub mod aws;
pub mod aws_config;
pub mod aws_sso;
pub mod aws_sts;
pub mod azure;
pub mod gcp;
pub mod netrc;
pub mod oauth2;
pub mod oauth2_flows;

pub use aws::{sha256_hex, sign_request, AwsSigV4Config};
pub use netrc::Netrc;
pub use oauth2::{get_token, refresh_token, CachedToken, OAuth2Config};
pub use oauth2_flows::{
    authorization_code_flow, device_flow, AuthCodeConfig, DeviceFlowConfig, OAuth2FlowType,
    PkceChallenge,
};
