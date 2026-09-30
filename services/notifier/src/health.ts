// `/healthz` (живий -- процес відповідає) / `/readyz` (є з'єднання з NATS і БД) — 09 §3.7.
// Простий `node:http`, без express/fastify заради однієї маршрутної пари.

import { createServer } from "node:http";
import { sql } from "./db.ts";
import type { NatsHandles } from "./nats.ts";

export function startHealthServer(port: number, nats: () => NatsHandles | null) {
  const server = createServer(async (req, res) => {
    if (req.url === "/healthz") {
      res.writeHead(200).end("ok");
      return;
    }
    if (req.url === "/readyz") {
      const handles = nats();
      const natsOk = handles !== null && !handles.nc.isClosed();
      let dbOk = false;
      try {
        await sql`SELECT 1`;
        dbOk = true;
      } catch {
        dbOk = false;
      }
      if (natsOk && dbOk) {
        res.writeHead(200).end("ok");
      } else {
        res.writeHead(503).end(JSON.stringify({ nats: natsOk, db: dbOk }));
      }
      return;
    }
    res.writeHead(404).end();
  });
  server.listen(port);
  return server;
}
