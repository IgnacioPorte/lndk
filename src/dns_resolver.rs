use crate::OfferError;
use bitcoin_payment_instructions::hrn_resolution::{HrnResolution, HrnResolver, HumanReadableName};
use bitcoin_payment_instructions::http_resolver::HTTPHrnResolver;
use std::sync::Arc;

#[cfg(itest)]
use std::sync::RwLock;

#[cfg(itest)]
pub static TEST_RESOLVER: RwLock<Option<Arc<dyn HrnResolver + Send + Sync>>> = RwLock::new(None);

pub struct LndkDNSResolverMessageHandler {
    resolver: Arc<dyn HrnResolver + Send + Sync>,
}

impl Default for LndkDNSResolverMessageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LndkDNSResolverMessageHandler {
    pub fn new() -> Self {
        #[cfg(itest)]
        {
            if let Ok(guard) = TEST_RESOLVER.read() {
                if let Some(test_resolver) = guard.as_ref() {
                    return Self {
                        resolver: Arc::clone(test_resolver),
                    };
                }
            }
        }
        Self::with_resolver(HTTPHrnResolver::new())
    }

    pub fn with_resolver<R: HrnResolver + Send + Sync + 'static>(resolver: R) -> Self {
        Self {
            resolver: Arc::new(resolver),
        }
    }

    pub async fn resolve_name_to_offer(&self, name_str: &str) -> Result<String, OfferError> {
        let resolved_uri = self.resolve_locally(name_str.to_string()).await?;
        self.extract_offer_from_uri(&resolved_uri)
    }

    pub fn extract_offer_from_uri(&self, uri: &str) -> Result<String, OfferError> {
        if let Some((_scheme, params)) = uri.split_once("?") {
            for param in params.split("&") {
                if let Some((key, value)) = param.split_once("=") {
                    if key.eq_ignore_ascii_case("lno") {
                        return Ok(value.to_string());
                    }
                }
            }
            Err(OfferError::ResolveUriError(
                "URI does not contain 'lno' parameter with BOLT12 offer".to_string(),
            ))
        } else {
            Err(OfferError::ResolveUriError(format!(
                "Invalid URI format - expected bitcoin:?lno=<offer>, got: {}",
                uri
            )))
        }
    }

    pub async fn resolve_locally(&self, name: String) -> Result<String, OfferError> {
        let hrn_parsed = HumanReadableName::from_encoded(&name)
            .map_err(|_| OfferError::ParseHrnFailure(name.clone()))?;

        let resolution = self
            .resolver
            .resolve_hrn(&hrn_parsed)
            .await
            .map_err(|e| OfferError::HrnResolutionFailure(format!("{}: {}", name, e)))?;

        let uri = match resolution {
            HrnResolution::DNSSEC { result, .. } => result,
            HrnResolution::LNURLPay { .. } => {
                return Err(OfferError::ResolveUriError(
                    "LNURL resolution not supported in this flow".to_string(),
                ))
            }
        };

        Ok(uri)
    }
}
