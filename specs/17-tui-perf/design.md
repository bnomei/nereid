# Design — 17-tui-perf

## Approach

### 1) Cache visible XRef indices in `App`

- Add `App.visible_xref_indices: Vec<usize>`.
- Add `App::recompute_visible_xref_indices(&mut self)` to populate the cache from:
  - `self.xrefs`
  - `self.xrefs_dangling_only`
- Change `visible_xref_indices()` to return `&[usize]` instead of allocating a new `Vec`.
- Update selection helpers to use the cached slice:
  - `selected_xref_index`
  - xref selection movement (prev/next/first/last)
  - `toggle_xrefs_dangling_only` (preserve prior selection when still visible)

### 2) Stop cloning labels into list widgets

- In `draw`, build list items from `&str`:
  - `ListItem::new(obj.label.as_str())`
  - `ListItem::new(xref.label.as_str())`
- Avoid cloning the selected xref label for the status bar (use `&str`).

## Notes / tradeoffs

- `ratatui::widgets::List` owns its items, so a `Vec<ListItem>` is still built each draw.
  - This spec focuses on eliminating *avoidable* string clones and repeated index-set allocations.

## Rollout

- Implement caching + borrow changes behind existing selection behavior.
- Rely on `src/tui/mod.rs` unit tests; add a targeted regression test only if selection semantics change.
