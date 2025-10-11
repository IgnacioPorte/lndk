use bitcoin_payment_instructions::hrn_resolution::{HrnResolution, HrnResolver, HumanReadableName};
use bitcoin_payment_instructions::http_resolver::HTTPHrnResolver;
use lightning::onion_message::dns_resolution::OMNameResolver;

pub struct LndkDNSResolverMessageHandler {
    om_resolver: OMNameResolver,
}

impl LndkDNSResolverMessageHandler {
    pub fn new(latest_block_time: u32, latest_block_height: u32) -> Self {
        Self {
            om_resolver: OMNameResolver::new(latest_block_time, latest_block_height),
        }
    }

    pub fn resolver(&self) -> &OMNameResolver {
        &self.om_resolver
    }

    pub async fn resolve_name_to_offer(
        &self,
        name_str: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resolved_uri = self.resolve_locally(name_str.to_string()).await?;

        self.extract_offer_from_uri(&resolved_uri).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                as Box<dyn std::error::Error + Send + Sync>
        })
    }

    pub fn extract_offer_from_uri(&self, uri: &str) -> Result<String, String> {
        if let Some((_scheme, params)) = uri.split_once("?") {
            for param in params.split("&") {
                if let Some((key, value)) = param.split_once("=") {
                    if key.eq_ignore_ascii_case("lno") {
                        return Ok(value.to_string());
                    }
                }
            }
            Err("URI does not contain 'lno' parameter with BOLT12 offer".to_string())
        } else {
            Err("Invalid URI format - expected bitcoin:?lno=<offer>".to_string())
        }
    }

    pub async fn resolve_locally(
        &self,
        name: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let resolver = HTTPHrnResolver::with_client(client);

        let hrn_parsed = HumanReadableName::from_encoded(&name).map_err(|_| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid human-readable name: {}", name),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let resolution = resolver.resolve_hrn(&hrn_parsed).await.map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HRN resolution failed for {}: {}", name, e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let uri = match resolution {
            HrnResolution::DNSSEC { result, .. } => result,
            HrnResolution::LNURLPay { .. } => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "LNURL resolution not supported in this flow",
                ))
                    as Box<dyn std::error::Error + Send + Sync>)
            }
        };

        Ok(uri)
    }
}
