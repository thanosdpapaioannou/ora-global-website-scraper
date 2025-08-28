use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fund {
    pub fund_name: String,
    pub fund_url: String,
    pub aum: String,
    pub linkedin_url: String,
    pub investment_geographies: String,
    pub fund_description: String,
    pub fund_portfolio: String,
}

impl Fund {
    pub fn new() -> Self {
        Self {
            fund_name: String::new(),
            fund_url: String::new(),
            aum: String::new(),
            linkedin_url: String::new(),
            investment_geographies: String::new(),
            fund_description: String::new(),
            fund_portfolio: String::new(),
        }
    }
}

impl Default for Fund {
    fn default() -> Self {
        Self::new()
    }
}