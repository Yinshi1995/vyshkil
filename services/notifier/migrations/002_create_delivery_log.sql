-- Журнал доставок (09 §3.7) -- номер МАСКУЄТЬСЯ перед записом (src/whatsapp/channel.ts), тут
-- лишається лише вже замаскований рядок -- ніколи повний номер, навіть у власній БД сервісу.
CREATE TABLE delivery_log (
    id serial PRIMARY KEY,
    org_id integer NOT NULL,
    phone_masked text NOT NULL,
    template text NOT NULL,
    status text NOT NULL, -- 'delivered' | 'failed' | 'suppressed' | 'stubbed'
    error text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_delivery_log_org_id ON delivery_log (org_id);
