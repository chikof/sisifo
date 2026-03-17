use types::{Result, SisiError, SiteHash};

pub enum SisiAddress {
    Hash(SiteHash),
    // TODO: Ipns(String), Handshake(String)
}

pub fn parse_address(addr: &str) -> Result<SisiAddress> {
    let stripped = addr
        .strip_prefix("sisi://")
        .or_else(|| addr.strip_prefix("ipfs://"))
        .unwrap_or(addr)
        .trim_end_matches('/');

    stripped
        .parse::<SiteHash>()
        .map(SisiAddress::Hash)
        .map_err(|_| SisiError::InvalidAddress(addr.to_string()))
}
