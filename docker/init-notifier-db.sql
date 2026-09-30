-- Виконується офіційним postgres-образом ОДИН раз, при першій ініціалізації тому даних
-- (docker-entrypoint-initdb.d/) -- окрема роль/БД для notifier (09-messaging.md §3.7: "власна
-- БД/схема в тому ж Postgres з окремим користувачем, не бачить таблиць застосунку"). Окрема
-- DATABASE (не лише схема) -- найпростіший спосіб гарантувати ізоляцію без ручних GRANT-правил.
CREATE ROLE notifier WITH LOGIN PASSWORD 'notifier';
CREATE DATABASE notifier OWNER notifier;
