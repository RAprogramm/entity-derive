-- Create products table for filtering example
CREATE TABLE IF NOT EXISTS products (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(100) NOT NULL,
    price BIGINT NOT NULL,
    stock INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for filtered columns
CREATE INDEX idx_products_category ON products(category);
CREATE INDEX idx_products_price ON products(price);
CREATE INDEX idx_products_active ON products(active);
CREATE INDEX idx_products_created_at ON products(created_at);

-- Sample data
INSERT INTO products (id, name, category, price, stock, active) VALUES
    (gen_random_uuid(), 'iPhone 15 Pro', 'electronics', 129900, 50, true),
    (gen_random_uuid(), 'MacBook Air M3', 'electronics', 149900, 30, true),
    (gen_random_uuid(), 'AirPods Pro', 'electronics', 24900, 100, true),
    (gen_random_uuid(), 'USB-C Cable', 'accessories', 1990, 500, true),
    (gen_random_uuid(), 'Phone Case', 'accessories', 2990, 200, true),
    (gen_random_uuid(), 'Old Phone Model', 'electronics', 49900, 5, false),
    (gen_random_uuid(), 'Wireless Charger', 'accessories', 4990, 75, true),
    (gen_random_uuid(), 'iPad Mini', 'electronics', 64900, 25, true);
