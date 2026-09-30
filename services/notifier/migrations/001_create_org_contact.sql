-- Контакти організацій для сповіщень (09-messaging.md §3.7) -- переїхало сюди з схеми застосунку
-- навмисно: notifier не бачить таблиць app, і навпаки (окремий Postgres-користувач/схема).
CREATE TABLE org_contact (
    id serial PRIMARY KEY,
    org_id integer NOT NULL,
    phone text NOT NULL,
    channel text NOT NULL DEFAULT 'whatsapp',
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_org_contact_org_id ON org_contact (org_id) WHERE active;
