---
name: reasoning
description: >
  Self-questioning checklist to run before implementing any feature.
  Use to surface edge cases, lifecycle gaps, persistence requirements,
  dedup needs, and UI completeness issues before writing code.
  Invoke mentally (or literally write answers) before starting implementation.
---

# Feature Reasoning Checklist

Before writing code, work through these question groups out loud.
Answer each one briefly — a one-liner is enough. Unanswered questions = gaps in the plan.

---

## 1. Data Model Questions

*Trigger: adding or changing any struct field, enum variant, or type.*

- Does this new field need to be **persisted** (saved to disk / TOML)?
  → If yes: add `#[serde(default)]` so old files still load.
- Are there **struct literal initializations** (`Struct { field: val, ... }`) elsewhere
  that will now fail to compile?
  → Search for every place the struct is constructed by name.
- Does the `Default` impl need updating?
- Do any `Clone`, `PartialEq`, or `Display` impls need to handle the new field?
- If I added an enum variant: are there `match` statements on this enum elsewhere?
  → Every exhaustive match must get a new arm.

---

## 2. Lifecycle / Sync Questions

*Trigger: any feature that creates, modifies, or destroys stateful data.*

- **When is this data created?** (startup, user action, event?)
- **When is it modified?** (every keystroke, on submit, on navigate-away?)
- **When should it be flushed/saved?** Never assume "it's saved automatically."
  → List every code path that changes it; each path needs a save call or a sync point.
- **When is it destroyed?** (tab close, workspace switch, app exit?)
  → Every destruction path must flush state first.
- What happens on **app restart**? Is the state loaded back correctly?
  → Test: create → close app → reopen → is state intact?

---

## 3. Idempotency / Dedup Questions

*Trigger: any feature that opens, creates, or adds something.*

- What happens if the user **triggers this action twice** in a row?
  → Should it be a no-op, an error, or create a second item?
- Is there an **existing open instance** that should be re-focused instead?
  → Search open_tabs / active list before pushing a new item.
- What if the **item already exists** with the same identity?
  → Define identity: is it by ID, by name, by position?

---

## 4. Inverse / Symmetric Operations

*Trigger: any "open", "create", "start", or "enable" action.*

For every action X, list its inverses and verify each handles the new state:

| Action added | Inverses to check |
|---|---|
| open tab | close tab, switch tab, switch workspace, app exit |
| create item | delete item, rename item, duplicate item |
| add field | all constructors, all serialization round-trips |
| enable feature | disable feature, reset to default |

**Every entry point needs a matching exit point that cleans up / persists.**

---

## 5. UI Completeness Questions

*Trigger: any new visible element (widget, panel, row, popup, indicator).*

- Should this element be **keyboard-navigable**?
  → If yes: add a `Focus` variant, update `next()`/`prev()`, add visual indicator.
- Does it need a **direct shortcut key** (number, letter)?
- Does it need **keybinding hints** in a status bar or hint footer?
- What happens when the element is **empty** (zero items, blank state)?
  → Render a placeholder; don't panic on `list[0]`.
- What happens when the element is **out of screen bounds** (many items, small terminal)?
  → Clamp scroll_offset; ensure cursor stays visible.

---

## 6. Related State Coherence

*Trigger: any action that adds or removes items from a list, or changes active indices.*

- After this action, are all **indices still valid**?
  → active_tab_idx, cursor, scroll_offset — clamp them after mutations.
- Are there **other state fields** that reference the mutated data by index or ID?
  → Update or invalidate caches (e.g. highlighted_body, selected row).
- Does any **other component render** based on the data I changed?
  → Read render functions that touch the same state; ensure they handle new shape.

---

## 7. The "Who Else Touches This?" Audit

*Run this for every struct, field, or function you modify.*

1. Search the codebase for all uses of the symbol.
2. Categorize: create / read / update / delete / display / persist / test.
3. For each category: does my change break or require updating that site?

```
Symbol: CollectionRequest
  create: CollectionRequest::new(), struct literal in sidebar_duplicate → needs url/body_raw
  read:   flatten_tree(), find_col_request_by_id() → fine, reads by field
  update: update_col_request_state() → needs url/body_raw
  persist: save_collection_meta() → handled by serde
  open:   handle_sidebar_enter() → needs to load url/body_raw back
  close:  close_active_tab() → needs to sync url/body_raw first
```

---

## Checklist (run before every implementation)

- [ ] Listed all struct literals for modified structs → all compile?
- [ ] Named every lifecycle stage (create / modify / destroy) and wired save/load
- [ ] Checked for dedup: what if this is triggered twice?
- [ ] Verified all inverses (close, delete, switch) handle the new state
- [ ] If new UI element: added to Focus cycle, visual indicator, hint bar
- [ ] Indices/cursors clamped after any list mutation
- [ ] "Who else touches this?" audit complete — no missed call sites
