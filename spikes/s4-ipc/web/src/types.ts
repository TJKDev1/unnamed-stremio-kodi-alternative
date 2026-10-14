// Mirrors core/src/vm.rs, typed.rs and generic.rs. In the app these would be generated.

export interface Card {
  id: string;
  title: string;
  poster: string;
  year: number | null;
  progress: number | null;
  badges: string[];
}

export interface Row {
  id: string;
  title: string;
  loading: boolean;
  cards: Card[];
}

export interface Home {
  rows: Row[];
}

export type Intent =
  | { MarkWatched: { card: string } }
  | { LoadMore: { row: string } }
  | { HideRow: { row: string } };

/** Typed ops, externally tagged (serde's default), as JSON and MessagePack deliver them. */
export type TypedOp =
  | { RowInsert: { index: number; row: Row } }
  | { RowRemove: { index: number } }
  | { RowMove: { from: number; to: number } }
  | { RowTitle: { row: number; title: string } }
  | { RowLoading: { row: number; loading: boolean } }
  | { RowCards: { row: number; cards: Card[] } }
  | { CardInsert: { row: number; index: number; card: Card } }
  | { CardRemove: { row: number; index: number } }
  | { CardMove: { row: number; from: number; to: number } }
  | { CardTitle: { row: number; card: number; title: string } }
  | { CardPoster: { row: number; card: number; poster: string } }
  | { CardYear: { row: number; card: number; year: number | null } }
  | { CardProgress: { row: number; card: number; progress: number | null } }
  | { CardBadges: { row: number; card: number; badges: string[] } }
  | { Home: { home: Home } };

export type Json = null | boolean | number | string | Json[] | { [k: string]: Json };
export type Seg = string | number;

export type GenericOp =
  | { Set: { path: Seg[]; value: Json } }
  | { Unset: { path: Seg[] } }
  | { Insert: { path: Seg[]; index: number; value: Json } }
  | { Remove: { path: Seg[]; index: number } }
  | { Move: { path: Seg[]; from: number; to: number } };

export type DiffKind = "generic" | "typed";
export type Encoding = "json" | "msgpack" | "postcard";
export type Scenario = "stream" | "refresh";
