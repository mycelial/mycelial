use std::{io::Cursor, sync::Arc};

use ::time::OffsetDateTime;
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequest, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose,
};
pub use rcgen::{Certificate, CertifiedKey, KeyPair};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{
        verify_tls12_signature, verify_tls13_signature, CryptoProvider, WebPkiSupportedAlgorithms,
    },
    server::danger::{ClientCertVerified, ClientCertVerifier},
    DigitallySignedStruct, OtherError,
};
pub use rustls::{ClientConfig, ServerConfig};
use rustls_pemfile::{certs, private_key};
pub use webpki::types::CertificateDer;
use webpki::types::{PrivateKeyDer, ServerName, TrustAnchor, UnixTime};
use x509_parser::prelude::*;

pub type Result<T, E = StdError> = std::result::Result<T, E>;
pub type StdError = Box<dyn std::error::Error + Send + Sync>;

pub fn generate_ca_certkey(name: &str) -> Result<CertifiedKey> {
    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::new([name.to_string()])?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.distinguished_name = {
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, name);
        distinguished_name
    };
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    params.key_usages.push(KeyUsagePurpose::CrlSign);
    params.not_before = OffsetDateTime::now_utc();
    let cert = params.self_signed(&key_pair)?;
    Ok(CertifiedKey { cert, key_pair })
}

pub fn generate_control_plane_cert(ca: &CertifiedKey, name: &str) -> Result<CertifiedKey> {
    let mut params = CertificateParams::new(vec![name.into()])?;
    params.distinguished_name.push(DnType::CommonName, name);
    params.use_authority_key_identifier_extension = true;
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ServerAuth);
    params.not_before = OffsetDateTime::now_utc();

    let key_pair = KeyPair::generate()?;
    let cert = params.signed_by(&key_pair, &ca.cert, &ca.key_pair)?;
    Ok(CertifiedKey { cert, key_pair })
}

pub fn generate_client_cert(ca: &CertifiedKey, name: &str) -> Result<CertifiedKey> {
    let mut params = CertificateParams::new(vec![name.into()])?;
    params.distinguished_name.push(DnType::CommonName, name);
    params.use_authority_key_identifier_extension = true;
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ClientAuth);
    params.not_before = OffsetDateTime::now_utc();

    let key_pair = KeyPair::generate()?;
    let cert = params.signed_by(&key_pair, &ca.cert, &ca.key_pair)?;
    Ok(CertifiedKey { cert, key_pair })
}

pub fn generate_csr_request(id: &str) -> Result<(KeyPair, CertificateSigningRequest)> {
    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::new(vec![id.to_string()])?;
    params.distinguished_name.push(DnType::CommonName, id);
    let csr = params.serialize_request(&key_pair)?;
    Ok((key_pair, csr))
}

/// rebuild ca certkey from certificate params/keypair
///
/// certificate will be parsed as a CertificateParams and 'fake' signed by ca key
/// rustls currently doesn't provide infrastracture to rebuild certificate:
/// [issue](https://github.com/rustls/rcgen/issues/274)
pub fn rebuild_ca_certkey(key: &str, cert: &str) -> Result<CertifiedKey> {
    let key_pair = KeyPair::from_pem(key)?;
    let cert_params = CertificateParams::from_ca_cert_pem(cert)?;
    let cert = cert_params.self_signed(&key_pair)?;
    Ok(CertifiedKey { cert, key_pair })
}

/// parse pem-serialized certificate
pub fn parse_certificate(cert: &str) -> Result<CertificateDer<'static>> {
    match certs(&mut Cursor::new(cert)).next() {
        Some(cert) => Ok(cert?),
        None => Err("fialed to parse certificate, input is empty")?,
    }
}

/// parse pem-serialized certificate
pub fn parse_keypair(keypair: &str) -> Result<KeyPair> {
    Ok(KeyPair::from_pem(keypair)?)
}

pub fn parse_private_key(keypair: &str) -> Result<PrivateKeyDer<'static>> {
    Ok(private_key(&mut Cursor::new(keypair))?.ok_or("no key found")?)
}

pub fn sign_csr(ca: &CertifiedKey, csr: &str) -> Result<Certificate> {
    let mut csr_params = rcgen::CertificateSigningRequestParams::from_pem(csr)?;
    csr_params
        .params
        .key_usages
        .push(rcgen::KeyUsagePurpose::DigitalSignature);
    csr_params.params.not_before = OffsetDateTime::now_utc();
    csr_params
        .params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ClientAuth);
    Ok(csr_params.signed_by(&ca.cert, &ca.key_pair)?)
}

#[derive(Debug)]
pub struct Verifier {
    ca: Vec<TrustAnchor<'static>>,
    signature_verification_algorithms: WebPkiSupportedAlgorithms,
}

// FIXME:
fn to_err(err: webpki::Error) -> rustls::Error {
    let err: StdError = format!("{err}").into();
    rustls::Error::Other(OtherError(Arc::from(err)))
}

// DNS name is not checked
// CRL is not supported (yet)
impl Verifier {
    pub fn new(cert: CertificateDer<'_>) -> Result<Self> {
        let provider =
            CryptoProvider::get_default().ok_or("failed to get default crypto provider")?;
        Ok(Self {
            ca: vec![webpki::anchor_from_trusted_cert(&cert)?.to_owned()],
            signature_verification_algorithms: provider.signature_verification_algorithms,
        })
    }
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = webpki::EndEntityCert::try_from(end_entity).map_err(to_err)?;
        cert.verify_for_usage(
            self.signature_verification_algorithms.all,
            self.ca.as_slice(),
            intermediates,
            now,
            webpki::KeyUsage::server_auth(),
            None,
            None,
        )
        .map_err(to_err)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.signature_verification_algorithms.supported_schemes()
    }
}

impl ClientCertVerifier for Verifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> std::result::Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        let cert = webpki::EndEntityCert::try_from(end_entity).map_err(to_err)?;
        cert.verify_for_usage(
            self.signature_verification_algorithms.all,
            self.ca.as_slice(),
            intermediates,
            now,
            webpki::KeyUsage::client_auth(),
            None,
            None,
        )
        .map_err(to_err)?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.signature_verification_algorithms.supported_schemes()
    }
}

pub fn extract_common_name<'a>(certificate: &'a CertificateDer<'a>) -> Result<&'a str> {
    let (_, certificate) = x509_parser::parse_x509_certificate(certificate).unwrap();
    for extension in certificate.extensions() {
        if let ParsedExtension::SubjectAlternativeName(SubjectAlternativeName { general_names }) =
            extension.parsed_extension()
        {
            let name = general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(name) => Some(*name),
                    _ => None,
                })
                .next();
            if let Some(name) = name {
                return Ok(name);
            }
        };
    }
    Err("common name not present")?
}
