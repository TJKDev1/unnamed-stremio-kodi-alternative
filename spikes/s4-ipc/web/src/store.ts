// T0.S4.5: the view-model as a SolidJS store, patched in place so only the touched cards
// re-render. A frame's ops apply inside one `produce`, so Solid flushes the DOM once per frame.

import { createStore, produce, unwrap } from "solid-js/store";

import type { Card, GenericOp, Home, Row, Seg, TypedOp } from "./types";

const [home, setHome] = createStore<Home>({ rows: [] });
export { home };

export function resetHome(): void {
  setHome({ rows: [] });
}

export function snapshot(): Home {
  return unwrap(home);
}

function move<T>(xs: T[], from: number, to: number): void {
  const [x] = xs.splice(from, 1);
  xs.splice(to, 0, x as T);
}

/**
 * Structural edits (insert/remove/move) go to a plain working copy of each touched array,
 * assigned back once at the end of the frame: one splice on a store array notifies every
 * index after it, so 450 moves in place cost ~20k signal writes. Field edits still go
 * through the store proxies, which the working copies hold, so only touched cards update.
 */
export function applyTyped(ops: TypedOp[]): void {
  setHome(
    produce((h) => {
      let rows: Row[] | null = null;
      const cardLists = new Map<Row, Card[]>();
      const rowsNow = () => rows ?? h.rows;
      const editRows = () => (rows ??= [...h.rows]);
      const row = (i: number) => {
        const r = rowsNow()[i];
        if (!r) throw new Error(`no row ${i}`);
        return r;
      };
      const editCards = (i: number) => {
        const r = row(i);
        let list = cardLists.get(r);
        if (!list) cardLists.set(r, (list = [...r.cards]));
        return list;
      };
      const card = (r: number, i: number) => {
        const owner = row(r);
        const c = (cardLists.get(owner) ?? owner.cards)[i];
        if (!c) throw new Error(`no card ${r}/${i}`);
        return c;
      };
      for (const op of ops) {
        if ("CardProgress" in op) {
          const o = op.CardProgress;
          card(o.row, o.card).progress = o.progress;
        } else if ("CardInsert" in op) {
          editCards(op.CardInsert.row).splice(op.CardInsert.index, 0, op.CardInsert.card);
        } else if ("CardRemove" in op) {
          editCards(op.CardRemove.row).splice(op.CardRemove.index, 1);
        } else if ("CardMove" in op) {
          move(editCards(op.CardMove.row), op.CardMove.from, op.CardMove.to);
        } else if ("CardBadges" in op) {
          const o = op.CardBadges;
          card(o.row, o.card).badges = o.badges;
        } else if ("CardTitle" in op) {
          const o = op.CardTitle;
          card(o.row, o.card).title = o.title;
        } else if ("CardPoster" in op) {
          const o = op.CardPoster;
          card(o.row, o.card).poster = o.poster;
        } else if ("CardYear" in op) {
          const o = op.CardYear;
          card(o.row, o.card).year = o.year;
        } else if ("RowInsert" in op) {
          editRows().splice(op.RowInsert.index, 0, op.RowInsert.row);
        } else if ("RowRemove" in op) {
          editRows().splice(op.RowRemove.index, 1);
        } else if ("RowMove" in op) {
          move(editRows(), op.RowMove.from, op.RowMove.to);
        } else if ("RowTitle" in op) {
          row(op.RowTitle.row).title = op.RowTitle.title;
        } else if ("RowLoading" in op) {
          row(op.RowLoading.row).loading = op.RowLoading.loading;
        } else if ("RowCards" in op) {
          editCards(op.RowCards.row).splice(0, Infinity, ...op.RowCards.cards);
        } else {
          rows = [...op.Home.home.rows];
          cardLists.clear();
        }
      }
      // Rows first: a row inserted this frame is a plain object until it's in the store.
      if (rows) h.rows = rows;
      for (const [r, list] of cardLists) r.cards = list;
    }),
  );
}

type Obj = Record<string | number, unknown>;

/** Same working-copy idea as `applyTyped`, keyed by the array being edited. */
export function applyGeneric(ops: GenericOp[]): void {
  setHome(
    produce((h) => {
      const copies = new Map<unknown, { parent: Obj; key: Seg; list: unknown[] }>();
      const step = (v: unknown, s: Seg): unknown => {
        const next = (v as Obj)[s];
        return copies.get(next)?.list ?? next;
      };
      const at = (path: Seg[], n: number): Obj => {
        let v: unknown = h;
        for (let i = 0; i < n; i++) v = step(v, path[i] as Seg);
        if (typeof v !== "object" || v === null) throw new Error(`bad path ${path.join("/")}`);
        return v as Obj;
      };
      const editArray = (path: Seg[]): unknown[] => {
        const parent = at(path, path.length - 1);
        const key = path[path.length - 1] as Seg;
        const current = parent[key];
        const existing = copies.get(current);
        if (existing) return existing.list;
        if (!Array.isArray(current)) throw new Error(`not an array: ${path.join("/")}`);
        const list = [...(current as unknown[])];
        copies.set(current, { parent, key, list });
        return list;
      };
      for (const op of ops) {
        if ("Set" in op) {
          const { path, value } = op.Set;
          if (path.length === 0) {
            copies.clear();
            h.rows = (value as unknown as Home).rows;
          } else {
            const parent = at(path, path.length - 1);
            const key = path[path.length - 1] as Seg;
            copies.delete(parent[key]);
            parent[key] = value;
          }
        } else if ("Insert" in op) {
          editArray(op.Insert.path).splice(op.Insert.index, 0, op.Insert.value);
        } else if ("Remove" in op) {
          editArray(op.Remove.path).splice(op.Remove.index, 1);
        } else if ("Move" in op) {
          move(editArray(op.Move.path), op.Move.from, op.Move.to);
        } else {
          const { path } = op.Unset;
          Reflect.deleteProperty(at(path, path.length - 1), path[path.length - 1] as Seg);
        }
      }
      for (const { parent, key, list } of copies.values()) parent[key] = list;
    }),
  );
}
