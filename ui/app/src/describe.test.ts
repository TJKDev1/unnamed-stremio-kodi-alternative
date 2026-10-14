import { expect, test } from "vitest";
import { describe } from "./describe.js";

test("describes the loaded skin", () => {
  expect(describe()).toBe("the app: skin default, sdk 0.0.0");
});
