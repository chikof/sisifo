#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub relay_url: Option<String>,
    pub pkarr_url: Option<String>,
    pub dns_origin: Option<String>,
}

impl NodeConfig {
    pub fn custom(relay_url: &str, pkarr_url: &str, dns_origin: &str) -> Self {
        Self {
            relay_url: Some(relay_url.to_string()),
            pkarr_url: Some(pkarr_url.to_string()),
            dns_origin: Some(dns_origin.to_string()),
        }
    }
}
