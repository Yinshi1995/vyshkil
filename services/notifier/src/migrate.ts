// Простий SQL-раннер міграцій (09 §3.7: "Міграції — прості SQL-файли з раннером") -- без ORM,
// той самий принцип мінімалізму, що решта сервісу. Застосовує `migrations/*.sql` за іменем
// файлу, трек застосованих -- таблиця `_migrations` у ТІЙ САМІЙ схемі нотифікатора.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import postgres from "postgres";
import { config } from "./config.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const migrationsDir = join(__dirname, "..", "migrations");

export async function runMigrations(sql: postgres.Sql): Promise<void> {
  await sql`
    CREATE TABLE IF NOT EXISTS _migrations (
      name text PRIMARY KEY,
      applied_at timestamptz NOT NULL DEFAULT now()
    )
  `;

  const applied = new Set((await sql`SELECT name FROM _migrations`).map((r) => r.name as string));

  const files = readdirSync(migrationsDir)
    .filter((f: string) => f.endsWith(".sql"))
    .sort();

  for (const file of files) {
    if (applied.has(file)) continue;
    const contents = readFileSync(join(migrationsDir, file), "utf-8");
    console.log(`migrate: застосовую ${file}`);
    await sql.begin(async (tx) => {
      await tx.unsafe(contents);
      await tx`INSERT INTO _migrations (name) VALUES (${file})`;
    });
  }
}

// Дозволяє і `node --experimental-strip-types src/migrate.ts` напряму (package.json's "migrate"
// script), і `import { runMigrations } from "./migrate.ts"` з тестів/index.ts. `pathToFileURL`,
// не ручна конкатенація `file://` -- на Windows `process.argv[1]` містить `\`-шляхи, пряма
// конкатенація ніколи не збігалась би з `import.meta.url` (яке завжди `/`-шляхи).
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const sql = postgres(config.databaseUrl);
  await runMigrations(sql);
  await sql.end();
  console.log("migrate: готово");
}
