-- Ідемпотентність споживача (09-messaging.md §3.2) -- дублікат (та сама доставка "щонайменше
-- один раз") пропускається, не обробляється вдруге.
CREATE TABLE inbox (
    message_id text PRIMARY KEY,
    processed_at timestamptz NOT NULL DEFAULT now()
);
