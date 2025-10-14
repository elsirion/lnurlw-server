use anyhow::{Result, anyhow};
use async_trait::async_trait;
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescriptionRef};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::fmt;

/// Newtype wrapper around Bolt11Invoice for convenience methods
#[derive(Debug, Clone)]
pub struct Invoice(Bolt11Invoice);

impl FromStr for Invoice {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Bolt11Invoice::from_str(s)
            .map(Self)
            .map_err(|e| anyhow!("Invalid invoice: {}", e))
    }
}

impl Invoice {
    pub fn amount_msats(&self) -> Result<u64> {
        self.0
            .amount_milli_satoshis()
            .ok_or_else(|| anyhow!("Invoice must have an amount"))
    }
    
    pub fn description(&self) -> Option<String> {
        match self.0.description() {
            Bolt11InvoiceDescriptionRef::Direct(desc) => Some(desc.to_string()),
            Bolt11InvoiceDescriptionRef::Hash(_) => None,
        }
    }
    
    pub fn payment_hash(&self) -> String {
        hex::encode(self.0.payment_hash().as_ref() as &[u8])
    }
    
    pub fn is_expired(&self) -> bool {
        // For now, assume invoices don't expire quickly during our mock testing
        // In a real implementation, you'd check against current time
        false
    }
    
    pub fn bolt11(&self) -> String {
        self.0.to_string()
    }
    
    pub fn inner(&self) -> &Bolt11Invoice {
        &self.0
    }
}

impl fmt::Display for Invoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub success: bool,
    pub preimage: Option<String>,
    pub error: Option<String>,
}

#[async_trait]
pub trait LightningBackend: Send + Sync {
    /// Pay a Lightning invoice after validation
    async fn pay_invoice(&self, invoice: &Invoice, expected_amount_msats: u64) -> Result<PaymentResult>;
    
    /// Get node info (balance, etc.)
    async fn get_info(&self) -> Result<NodeInfo>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub alias: String,
    pub balance_msats: u64,
}

/// Mock implementation for testing
pub struct MockLightning;

#[async_trait]
impl LightningBackend for MockLightning {
    async fn pay_invoice(&self, invoice: &Invoice, expected_amount_msats: u64) -> Result<PaymentResult> {
        let amount_msats = invoice.amount_msats()?;
        
        if amount_msats != expected_amount_msats {
            return Ok(PaymentResult {
                success: false,
                preimage: None,
                error: Some(format!(
                    "Invoice amount {} msats doesn't match expected {} msats",
                    amount_msats, expected_amount_msats
                )),
            });
        }
        
        if invoice.is_expired() {
            return Ok(PaymentResult {
                success: false,
                preimage: None,
                error: Some("Invoice is expired".to_string()),
            });
        }
        
        // Mock successful payment
        Ok(PaymentResult {
            success: true,
            preimage: Some("0".repeat(64)),
            error: None,
        })
    }
    
    async fn get_info(&self) -> Result<NodeInfo> {
        Ok(NodeInfo {
            alias: "Mock Node".to_string(),
            balance_msats: 1_000_000_000,
        })
    }
}