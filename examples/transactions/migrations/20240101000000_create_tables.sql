-- SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
-- SPDX-License-Identifier: MIT

-- Create tables for transactions example

CREATE TABLE IF NOT EXISTS bank_accounts (
    id UUID PRIMARY KEY,
    owner_name VARCHAR(255) NOT NULL,
    balance BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS transfer_logs (
    id UUID PRIMARY KEY,
    from_account_id UUID NOT NULL REFERENCES bank_accounts(id),
    to_account_id UUID NOT NULL REFERENCES bank_accounts(id),
    amount BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_transfer_logs_from ON transfer_logs(from_account_id);
CREATE INDEX idx_transfer_logs_to ON transfer_logs(to_account_id);

-- Sample accounts with initial balances
INSERT INTO bank_accounts (id, owner_name, balance) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'Alice', 100000),
    ('a0000000-0000-0000-0000-000000000002', 'Bob', 50000),
    ('a0000000-0000-0000-0000-000000000003', 'Charlie', 75000);
