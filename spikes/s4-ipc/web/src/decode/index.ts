import { decode as decodeMsgpack } from "@msgpack/msgpack";

import type { DiffKind, Encoding, GenericOp, TypedOp } from "../types";
import { decodeGeneric, decodeTyped } from "./postcard";

const utf8 = new TextDecoder();

/** Decodes a frame payload. JSON and MessagePack give the same objects (see wire.rs tests). */
export function decode(diff: DiffKind, enc: Encoding, payload: Uint8Array): TypedOp[] | GenericOp[] {
  switch (enc) {
    case "json":
      return JSON.parse(utf8.decode(payload)) as TypedOp[] | GenericOp[];
    case "msgpack":
      return decodeMsgpack(payload) as TypedOp[] | GenericOp[];
    case "postcard":
      return diff === "typed" ? decodeTyped(payload) : decodeGeneric(payload);
  }
}
