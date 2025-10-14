-- Initial database schema for LNURLw server

CREATE TABLE IF NOT EXISTS cards (
    card_id INTEGER PRIMARY KEY AUTOINCREMENT,
    uid TEXT NOT NULL DEFAULT '',
    k0_auth_key TEXT NOT NULL,
    k1_decrypt_key TEXT NOT NULL,
    k2_cmac_key TEXT NOT NULL,
    k3 TEXT NOT NULL,
    k4 TEXT NOT NULL,
    last_counter INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    tx_limit_sats INTEGER NOT NULL,
    day_limit_sats INTEGER NOT NULL,
    card_name TEXT NOT NULL,
    one_time_code TEXT UNIQUE,
    one_time_code_expiry DATETIME,
    one_time_code_used BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS card_payments (
    payment_id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id INTEGER NOT NULL,
    k1 TEXT UNIQUE NOT NULL,
    invoice TEXT,
    amount_msats INTEGER,
    paid BOOLEAN DEFAULT 0,
    payment_time DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (card_id) REFERENCES cards(card_id)
);

CREATE INDEX IF NOT EXISTS idx_cards_uid ON cards(uid);
CREATE INDEX IF NOT EXISTS idx_cards_one_time_code ON cards(one_time_code);
CREATE INDEX IF NOT EXISTS idx_payments_k1 ON card_payments(k1);
CREATE INDEX IF NOT EXISTS idx_payments_card_id ON card_payments(card_id);