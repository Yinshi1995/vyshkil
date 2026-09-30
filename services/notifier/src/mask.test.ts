import { strict as assert } from "node:assert";
import { test } from "node:test";
import { maskPhone } from "./mask.ts";

test("matches the exact example from 09-messaging.md §5", () => {
  assert.equal(maskPhone("+380501234567"), "+380*****4567");
});

test("short input is fully masked, not partially leaked", () => {
  assert.equal(maskPhone("12345"), "*****");
});
