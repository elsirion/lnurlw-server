# LNURLw Server for Bolt Cards

A Rust implementation of a Bolt Card compatible LNURLw server with SQLite database backend.

## Features

- ✅ **LNURLw Protocol**: Full LNURLw withdraw support with Bolt Card compatibility
- ✅ **Per-Card Security**: Each card has unique AES keys (k0, k1, k2, k3, k4)
- ✅ **Replay Protection**: Counter-based protection against replay attacks
- ✅ **Card Registration**: One-time code system for secure card programming
- ✅ **Payment Limits**: Per-transaction and daily spending limits
- ✅ **SQLite Backend**: Lightweight database with migrations
- ✅ **Type Safety**: Newtype wrappers for cryptographic primitives
- ✅ **Lightning Integration**: Abstract Lightning backend trait (Mock implementation included)

## Usage

### Running the Server

```bash
# With required domain parameter
cargo run -- --domain cards.example.com

# With custom configuration
cargo run -- \
  --domain cards.example.com \
  --host 127.0.0.1 \
  --port 3000 \
  --database-url sqlite://my-cards.db \
  --default-tx-limit 50000 \
  --default-day-limit 500000
```

### Environment Variables

All CLI arguments can also be set via environment variables:

```bash
export DOMAIN=cards.example.com
export HOST=0.0.0.0
export PORT=8080
export DATABASE_URL=sqlite://lnurlw.db
export DEFAULT_TX_LIMIT=100000
export DEFAULT_DAY_LIMIT=1000000

cargo run
```

## API Endpoints

### Card Management

#### Create New Card
```http
POST /api/createboltcard
Content-Type: application/json

{
  "card_name": "My Card",
  "tx_limit_sats": 50000,
  "day_limit_sats": 500000,
  "enabled": true
}
```

Response:
```json
{
  "status": "OK",
  "url": "https://cards.example.com/new?a=abc123..."
}
```

#### Get Card Configuration
```http
GET /new?a=abc123...
```

Response (for NFC programming):
```json
{
  "protocol_name": "create_bolt_card_response",
  "protocol_version": 2,
  "card_name": "My Card",
  "lnurlw_base": "lnurlw://cards.example.com/ln",
  "k0": "...",
  "k1": "...",
  "k2": "...",
  "k3": "...",
  "k4": "..."
}
```

### LNURLw Protocol

#### Initial Request
```http
GET /ln?card_id=<card_id>&p=<encrypted_data>&c=<cmac>
```

#### Callback
```http
GET /ln/callback?k1=<session_key>&pr=<lightning_invoice>
```

## Protocol Flow

1. **Card Creation**: Admin creates card via API, receives one-time registration URL
2. **Card Programming**: Card is programmed with unique keys using NFC app and registration URL
3. **Payment**: Card is tapped on POS terminal
   - POS reads encrypted LNURL from card
   - Server decrypts and validates using card's unique keys
   - Server checks counter for replay protection
   - Server returns LNURLw response with payment limits
   - POS generates Lightning invoice and calls callback
   - Server pays invoice via Lightning backend

## Security Features

- **Per-Card Keys**: No shared secrets between cards
- **Counter-Based Replay Protection**: Prevents card tap replay attacks
- **Payment Limits**: Transaction and daily limits per card
- **One-Time Registration**: Registration URLs expire after use
- **CMAC Authentication**: Tamper-proof card authentication

## Database Schema

The server uses SQLite with two main tables:

- `cards`: Stores card information, keys, limits, and counters
- `card_payments`: Tracks payment history and Lightning invoices

Migrations are automatically applied on startup.

## Development

### Adding Lightning Backend

Implement the `LightningBackend` trait:

```rust
use async_trait::async_trait;
use crate::lightning::{LightningBackend, Invoice, PaymentResult, NodeInfo};

pub struct MyLightningBackend;

#[async_trait]
impl LightningBackend for MyLightningBackend {
    async fn pay_invoice(&self, invoice: &Invoice, expected_amount_msats: u64) -> Result<PaymentResult> {
        // Implement Lightning payment logic
    }

    async fn get_info(&self) -> Result<NodeInfo> {
        // Implement node info retrieval
    }
}
```

### Testing

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Build release version
cargo build --release
```

## Architecture

- **Axum**: Web framework for HTTP endpoints
- **SQLx**: Type-safe SQL queries with compile-time verification
- **Tokio**: Async runtime
- **Clap**: CLI argument parsing
- **Lightning-Invoice**: Invoice parsing and validation
- **AES + CMAC**: Cryptographic operations for card validation

## License

This project follows the same security model as the reference Bolt Card implementation while being implemented in Rust for better performance and type safety.