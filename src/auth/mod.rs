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

pub use aws::{AwsSigV4Config, sign_request, sha256_hex};
pub use netrc::Netrc;
pub use oauth2::{OAuth2Config, get_token, CachedToken, refresh_token};
pub use oauth2_flows::{
    AuthCodeConfig, DeviceFlowConfig, PkceChallenge, OAuth2FlowType,
    authorization_code_flow, device_flow,
};
