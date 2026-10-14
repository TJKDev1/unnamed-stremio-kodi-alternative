// Hand-written postcard decoders. Postcard is positional: these mirror the Rust field and
// variant order exactly (vm.rs, typed.rs, wire.rs). A schema compiler would generate them.
//
// Wire rules: u8 and bool are one byte; u16/u32 are LEB128 varints; i64 is a zigzag varint;
// f64 is 8 bytes LE; Option is a 0/1 byte then the value; Vec and String are a varint
// length then the items / UTF-8 bytes; enums are a varint variant index then the fields.

import type { Card, GenericOp, Home, Json, Row, Seg, TypedOp } from "../types";

const utf8 = new TextDecoder();

class Reader {
  pos = 0;
  constructor(private readonly b: Uint8Array) {}

  u8(): number {
    const x = this.b[this.pos++];
    if (x === undefined) throw new Error("postcard: unexpected end");
    return x;
  }

  bool(): boolean {
    return this.u8() === 1;
  }

  varint(): number {
    let result = 0;
    let scale = 1;
    for (;;) {
      const x = this.u8();
      result += (x & 0x7f) * scale;
      if (x < 0x80) return result;
      scale *= 128;
    }
  }

  zigzag(): number {
    const n = this.varint();
    return n % 2 === 0 ? n / 2 : -(n + 1) / 2;
  }

  f64(): number {
    const v = new DataView(this.b.buffer, this.b.byteOffset + this.pos, 8).getFloat64(0, true);
    this.pos += 8;
    return v;
  }

  str(): string {
    const n = this.varint();
    const start = this.pos;
    this.pos += n;
    if (this.pos > this.b.length) throw new Error("postcard: unexpected end");
    // Short ASCII strings (ids, titles) are faster to build by hand than via TextDecoder.
    if (n < 64) {
      let s = "";
      for (let i = start; i < start + n; i++) {
        const c = this.b[i] as number;
        if (c >= 0x80) return utf8.decode(this.b.subarray(start, start + n));
        s += String.fromCharCode(c);
      }
      return s;
    }
    return utf8.decode(this.b.subarray(start, start + n));
  }

  opt<T>(f: () => T): T | null {
    return this.bool() ? f() : null;
  }

  seq<T>(f: () => T): T[] {
    const n = this.varint();
    const out: T[] = [];
    for (let i = 0; i < n; i++) out.push(f());
    return out;
  }

  done(): void {
    if (this.pos !== this.b.length) {
      throw new Error(`postcard: ${this.b.length - this.pos} trailing bytes`);
    }
  }
}

function card(r: Reader): Card {
  return {
    id: r.str(),
    title: r.str(),
    poster: r.str(),
    year: r.opt(() => r.varint()),
    progress: r.opt(() => r.u8()),
    badges: r.seq(() => r.str()),
  };
}

function row(r: Reader): Row {
  return { id: r.str(), title: r.str(), loading: r.bool(), cards: r.seq(() => card(r)) };
}

function home(r: Reader): Home {
  return { rows: r.seq(() => row(r)) };
}

function typedOp(r: Reader): TypedOp {
  const tag = r.varint();
  switch (tag) {
    case 0:
      return { RowInsert: { index: r.varint(), row: row(r) } };
    case 1:
      return { RowRemove: { index: r.varint() } };
    case 2:
      return { RowMove: { from: r.varint(), to: r.varint() } };
    case 3:
      return { RowTitle: { row: r.varint(), title: r.str() } };
    case 4:
      return { RowLoading: { row: r.varint(), loading: r.bool() } };
    case 5:
      return { RowCards: { row: r.varint(), cards: r.seq(() => card(r)) } };
    case 6:
      return { CardInsert: { row: r.varint(), index: r.varint(), card: card(r) } };
    case 7:
      return { CardRemove: { row: r.varint(), index: r.varint() } };
    case 8:
      return { CardMove: { row: r.varint(), from: r.varint(), to: r.varint() } };
    case 9:
      return { CardTitle: { row: r.varint(), card: r.varint(), title: r.str() } };
    case 10:
      return { CardPoster: { row: r.varint(), card: r.varint(), poster: r.str() } };
    case 11:
      return { CardYear: { row: r.varint(), card: r.varint(), year: r.opt(() => r.varint()) } };
    case 12:
      return { CardProgress: { row: r.varint(), card: r.varint(), progress: r.opt(() => r.u8()) } };
    case 13:
      return { CardBadges: { row: r.varint(), card: r.varint(), badges: r.seq(() => r.str()) } };
    case 14:
      return { Home: { home: home(r) } };
    default:
      throw new Error(`postcard: unknown typed op ${tag}`);
  }
}

export function decodeTyped(b: Uint8Array): TypedOp[] {
  const r = new Reader(b);
  const ops = r.seq(() => typedOp(r));
  r.done();
  return ops;
}

function path(r: Reader): Seg[] {
  return r.seq(() => (r.varint() === 0 ? r.str() : r.varint()));
}

function value(r: Reader): Json {
  const tag = r.varint();
  switch (tag) {
    case 0:
      return null;
    case 1:
      return r.bool();
    case 2:
      return r.zigzag();
    case 3:
      return r.f64();
    case 4:
      return r.str();
    case 5:
      return r.seq(() => value(r));
    case 6: {
      const o: { [k: string]: Json } = {};
      const n = r.varint();
      for (let i = 0; i < n; i++) {
        const k = r.str();
        o[k] = value(r);
      }
      return o;
    }
    default:
      throw new Error(`postcard: unknown value tag ${tag}`);
  }
}

function genericOp(r: Reader): GenericOp {
  const tag = r.varint();
  switch (tag) {
    case 0:
      return { Set: { path: path(r), value: value(r) } };
    case 1:
      return { Unset: { path: path(r) } };
    case 2:
      return { Insert: { path: path(r), index: r.varint(), value: value(r) } };
    case 3:
      return { Remove: { path: path(r), index: r.varint() } };
    case 4:
      return { Move: { path: path(r), from: r.varint(), to: r.varint() } };
    default:
      throw new Error(`postcard: unknown generic op ${tag}`);
  }
}

export function decodeGeneric(b: Uint8Array): GenericOp[] {
  const r = new Reader(b);
  const ops = r.seq(() => genericOp(r));
  r.done();
  return ops;
}
