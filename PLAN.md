# Implementation Plan for Bolt Card LNURLw Server

## Architecture Overview
- **Web Framework**: Axum for HTTP endpoints
- **Database**: SQLite with sqlx for async operations
- **Runtime**: Tokio
- **Configuration**: Clap for CLI arguments (no settings table needed)
- **Cryptography**: Native Rust crypto libraries (aes, cmac)

## Core Components

### Database Schema (SQLite)
```sql
-- Cards table for bolt card management
CREATE TABLE cards (
    card_id INTEGER PRIMARY KEY AUTOINCREMENT,
    uid TEXT DEFAULT '',
    k0_auth_key TEXT NOT NULL,     -- 32 hex chars (per-card)
    k1_decrypt_key TEXT NOT NULL,  -- 32 hex chars (per-card)
    k2_cmac_key TEXT NOT NULL,     -- 32 hex chars (per-card)
    k3 TEXT NOT NULL,               -- 32 hex chars (per-card, NXP requirement)
    k4 TEXT NOT NULL,               -- 32 hex chars (per-card, NXP requirement)
    last_counter INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    tx_limit_sats INTEGER NOT NULL,
    day_limit_sats INTEGER NOT NULL,
    card_name TEXT NOT NULL,
    uid_privacy BOOLEAN NOT NULL DEFAULT 0,
    one_time_code TEXT UNIQUE,
    one_time_code_expiry DATETIME,
    one_time_code_used BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Payment tracking
CREATE TABLE card_payments (
    payment_id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id INTEGER NOT NULL,
    k1 TEXT UNIQUE NOT NULL,     -- LNURLw k1 parameter (for withdrawal)
    invoice TEXT,
    amount_msats INTEGER,
    paid BOOLEAN DEFAULT 0,
    payment_time DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (card_id) REFERENCES cards(card_id)
);
```

## Endpoints

### A. Card Registration Endpoint
`GET /new?a={one_time_code}`
- Returns card configuration for NFC programming
- Response includes all card-specific keys and lnurlw base URL
- One-time code expires after use
- Returns: k0, k1, k2, k3, k4 (all unique per card)

### B. Internal API for Card Creation
`POST /api/createboltcard`
- Creates new card record with randomly generated keys
- Returns registration URL with one-time code
- Parameters: card_name, tx_max, day_max, enabled, uid_privacy

### C. LNURLw Endpoint
`GET /ln?card_id={card_id}&p={encrypted_data}&c={cmac}`
- Decrypts UID and counter from `p` parameter using card-specific k1
- Validates CMAC authentication using card-specific k2
- Checks counter is increasing (replay protection)
- Returns LNURLw withdraw response

### D. LNURLw Callback
`GET /ln/callback?k1={k1}&pr={invoice}`
- Processes withdrawal with provided invoice
- Validates k1 and payment limits
- Initiates Lightning payment

## Cryptographic Operations

### Key Components:
- **Per-Card AES Decryption**: Each card has unique k1 for decrypting `p` parameter
- **Per-Card AES-CMAC**: Each card has unique k2 for validating `c` parameter
- **Counter Validation**: Ensure counter increases (replay protection)

### Security Model:
- All keys are per-card (no shared secrets)
- Compromising one card doesn't affect others
- Each card tap produces unique URL via counter
- CMAC prevents tampering, counter prevents replay

## Configuration Structure (via Clap)
```rust
struct Config {
    // Server settings
    host: String,           // default: "0.0.0.0"
    port: u16,             // default: 8080
    domain: String,        // public domain for lnurlw URLs

    // Database
    database_url: String,  // SQLite path
}
```

## Project Structure
```
src/
├── main.rs           # Entry point, server setup
├── config.rs         # Clap configuration
├── db/
│   ├── mod.rs       # Database module
│   ├── models.rs    # SQLx models
│   └── queries.rs   # Database operations
├── crypto/
│   ├── mod.rs       # Crypto operations
│   ├── aes.rs       # AES decrypt/encrypt
│   └── cmac.rs      # CMAC verification
├── handlers/
│   ├── mod.rs       # HTTP handlers
│   ├── lnurlw.rs    # LNURLw endpoints
│   └── register.rs  # Card registration
├── lightning/
│   └── mod.rs       # Lightning backend interface
└── utils.rs         # Helpers (hex encoding, etc)
```

## Dependencies (Cargo.toml)
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
hex = "0.4"
aes = "0.8"
cmac = "0.7"
cipher = "0.4"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
rand = "0.8"  # For key generation
```

## Key Implementation Notes

1. **No Settings Table**: All configuration via CLI args, all secrets are per-card
2. **Per-Card Keys**: Each card has unique k0, k1, k2, k3, k4 keys
3. **Privacy**: Support UID privacy mode where card UID is discovered via CMAC matching
4. **Security**: All keys are 16-byte (32 hex chars) randomly generated per card
5. **Replay Protection**: Counter must strictly increase, store last seen value
6. **Payment Limits**: Enforce per-transaction and daily limits
7. **Lightning Integration**: Abstract interface for LND/CLN/LNbits backends

## Protocol Flow

1. **Card Registration**:
   - Admin creates card via API with limits
   - System generates random per-card keys (k0, k1, k2, k3, k4)
   - One-time code provided for card programming
   - Card programmed with its unique keys via NFC app

2. **Payment Flow**:
   - Card tapped on POS terminal
   - POS reads NDEF message: `lnurlw://domain/ln?card_id={card_id}&p={encrypted}&c={cmac}`
   - Server looks up card by ID and decrypts `p` with that card's k1
   - Server validates CMAC with that card's k2
   - Server checks counter > last_counter for that card
   - Server returns LNURLw response with callback URL
   - POS generates invoice and calls callback
   - Server pays invoice via Lightning backend (model it as a trait, not a specific implementation)

3. **Security Advantages**:
   - No shared secrets between cards
   - Card compromise doesn't affect other cards
   - Each tap produces unique URL (counter-based)
   - CMAC prevents tampering
   - Counter prevents replay attacks
   - One-time codes expire after use

## UID Privacy Mode
Out of scope for now