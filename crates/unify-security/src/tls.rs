use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls13_signature, CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{CertificateError, ClientConfig, DigitallySignedStruct, DistinguishedName, Error, ServerConfig, SignatureScheme};

use crate::identity::{DeviceIdentity, Fingerprint};
use crate::SecurityError;

pub const ALPN_PROTOCOL: &[u8] = b"unify/1";
pub const PAIRING_EXPORTER_LABEL: &[u8] = b"EXPORTER-unify-pairing-v1";
pub const SERVER_NAME: &str = "unify.local";

pub fn crypto_provider() -> CryptoProvider {
    rustls::crypto::ring::default_provider()
}

fn algorithms() -> WebPkiSupportedAlgorithms {
    crypto_provider().signature_verification_algorithms
}

fn check_tls13_signature(message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
    verify_tls13_signature(message, cert, dss, &algorithms())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerExpectation {
    Pinned(Fingerprint),
    AnyForPairing,
}

#[derive(Debug)]
struct PinnedServerVerifier {
    expectation: ServerExpectation,
}

impl ServerCertVerifier for PinnedServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if !intermediates.is_empty() {
            return Err(Error::InvalidCertificate(CertificateError::BadEncoding));
        }
        match self.expectation {
            ServerExpectation::AnyForPairing => Ok(ServerCertVerified::assertion()),
            ServerExpectation::Pinned(expected) if Fingerprint::of(end_entity) == expected => Ok(ServerCertVerified::assertion()),
            ServerExpectation::Pinned(_) => Err(Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure)),
        }
    }

    fn verify_tls12_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General(String::from("TLS 1.2 is disabled")))
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        check_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        algorithms().supported_schemes()
    }
}

#[derive(Debug)]
struct ProofOfPossessionClientVerifier;

impl ClientCertVerifier for ProofOfPossessionClientVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(&self, _end_entity: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], _now: UnixTime) -> Result<ClientCertVerified, Error> {
        if !intermediates.is_empty() {
            return Err(Error::InvalidCertificate(CertificateError::BadEncoding));
        }
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, _message: &[u8], _cert: &CertificateDer<'_>, _dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        Err(Error::General(String::from("TLS 1.2 is disabled")))
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        check_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        algorithms().supported_schemes()
    }
}

pub fn server_config(identity: &DeviceIdentity) -> Result<Arc<ServerConfig>, SecurityError> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(crypto_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(Arc::new(ProofOfPossessionClientVerifier))
        .with_single_cert(vec![identity.certificate().clone()], identity.private_key())?;
    config.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];
    config.max_early_data_size = 0;
    config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
    config.send_tls13_tickets = 0;
    Ok(Arc::new(config))
}

pub fn client_config(identity: &DeviceIdentity, expectation: ServerExpectation) -> Result<Arc<ClientConfig>, SecurityError> {
    let mut config = ClientConfig::builder_with_provider(Arc::new(crypto_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedServerVerifier { expectation }))
        .with_client_auth_cert(vec![identity.certificate().clone()], identity.private_key())?;
    config.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];
    config.enable_early_data = false;
    config.resumption = rustls::client::Resumption::disabled();
    Ok(Arc::new(config))
}

pub fn server_name() -> ServerName<'static> {
    ServerName::try_from(SERVER_NAME).unwrap_or_else(|_| ServerName::IpAddress(std::net::Ipv4Addr::LOCALHOST.into()))
}

pub fn peer_fingerprint(state: &rustls::CommonState) -> Option<(Fingerprint, Vec<u8>)> {
    let certificates = state.peer_certificates()?;
    if certificates.len() != 1 {
        return None;
    }
    let der = certificates[0].as_ref().to_vec();
    Some((Fingerprint::of(&der), der))
}
