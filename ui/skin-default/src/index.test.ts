import { expect, test } from "vitest";
import { skin } from "./index.js";

test("default skin targets the SDK", () => {
  expect(skin.id).toBe("default");
  expect(skin.sdkVersion).toBe("0.0.0");
});
