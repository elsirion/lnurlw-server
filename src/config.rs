use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "lnurlw-server")]
#[command(about = "Bolt Card compatible LNURLw server")]
#[command(version)]
pub struct Config {
    /// Host address to bind to
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,
    
    /// Port to listen on
    #[arg(long, env = "PORT", default_value = "8080")]
    pub port: u16,
    
    /// Public domain for LNURLw URLs (e.g., "cards.example.com")
    #[arg(long, env = "DOMAIN")]
    pub domain: String,
    
    /// SQLite database URL
    #[arg(long, env = "DATABASE_URL", default_value = "sqlite://lnurlw.db")]
    pub database_url: String,
    
    /// Default transaction limit in satoshis
    #[arg(long, env = "DEFAULT_TX_LIMIT", default_value = "100000")]
    pub default_tx_limit: u64,
    
    /// Default daily limit in satoshis
    #[arg(long, env = "DEFAULT_DAY_LIMIT", default_value = "1000000")]
    pub default_day_limit: u64,
}

impl Config {
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
    
    pub fn lnurlw_base(&self) -> String {
        format!("lnurlw://{}/ln", self.domain)
    }
    
    pub fn registration_base(&self) -> String {
        format!("https://{}/new", self.domain)
    }
}