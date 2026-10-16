# rust-calc — Windows-style Calculator in Rust

Implementation plan. Decisions locked: **egui + eframe**, **Standard mode only**, **f64 math + decimal string display**.

---

## 1. Goal

Desktop calculator for Windows that behaves like the built-in Windows 11 Calculator in **Standard** mode:
same button layout, same display layout, same keyboard shortcuts, same chaining/equals/memory/history rules.

**Non-goals for v1** (explicitly skipped): Scientific, Programmer, Date, Converter, Graphing, button-flyout
unit conversion, always-on-top, theme switching beyond one dark Fluent-ish theme, localization, MS Store packaging.

---

## 2. Stack

| Concern | Choice | Why |
|---|---|---|
| UI | `eframe = "0.37"` (pulls `egui 0.37`) | one `cargo build`, no WebView, no Node |
| Renderer | default (`glow`) | wgpu not needed for a flat 20-button grid |
| Math | `f64` | decided |
| Formatting | hand-rolled grouping (no ICU crate) | thousands separators = one loop |
| Errors | own `enum CalcError` | no `anyhow` in a GUI with no I/O |
| Release size / speed | `opt-level = "s"`, `lto = true`, `strip = true`, `windows` subsystem | ~3–6 MB exe |

Windows-only APIs: none. Pure Rust, cross-compiles to Linux/Web too.

---

## 3. Architecture

Two layers, hard boundary. `core` has **zero** egui imports → unit-testable with `cargo test`, no window.

```
rust-calc/
├── Cargo.toml
├── PLAN.md
├── src/
│   ├── main.rs            # eframe entry, NativeOptions, window icon
│   ├── app.rs             # CalcApp: holds Core + UiState, update()
│   ├── core/
│   │   ├── mod.rs
│   │   ├── calc.rs        # Calculator state machine (the product)
│   │   ├── eval.rs        # apply_op(a, op, b)
│   │   ├── fmt.rs         # f64 -> display string, grouping, 16-digit cap
│   │   └── error.rs       # CalcError + user-facing message text
│   └── ui/
│       ├── mod.rs
│       ├── theme.rs       # dark palette, fonts, button styles
│       ├── display.rs     # expression line + main display
│       ├── keypad.rs      # 4x7 button grid, responsive sizing
│       ├── memory_bar.rs  # MC MR M+ M- MS
│       └── history.rs     # side panel, Ctrl+H
└── tests/
    └── parity.rs          # table-driven Windows-parity cases
```

Data flow: `input event → Core::input(Input) → Core state → render`. No callbacks, no message bus —
immediate mode already is the bus.

---

## 4. Core state machine (the real work)

```rust
pub enum Input { Digit(char), Dot, Add, Sub, Mul, Div, Percent,
                 Inv, Sqr, Sqrt, Negate, Equals, Clear, ClearEntry, Backspace }

pub struct Calculator {
    display: String,        // what the display shows, kept as string (input buffer)
    acc: Option<f64>,       // left operand / accumulator
    pending: Option<Op>,    // operator waiting for right operand
    last_op: Option<(Op, f64)>, // for Equals-repeat
    fresh: bool,            // next digit replaces display
    memory: f64,
    history: Vec<Entry>,    // newest first, cap 100
    error: Option<CalcError>,
}
```

State rules (this table **is** the spec — implement exactly, then test each row):

| Situation | Input | Result |
|---|---|---|
| typing | `Digit` | append if `display.len() < 16` sig digits, else ignore + no-op |
| after result | `Digit` | `fresh` → replace display |
| `2 + 3 +` | `Add` | evaluate `2+3` → display `5`, `pending=Add`, `acc=5` |
| `2 + *` | `Mul` | **replace** pending op, do not evaluate |
| `=` | `Equals` | `acc op b` → push history `2 + 3 =`; set `last_op` |
| `=` again | `Equals` | re-apply `last_op` → `5+3=8`, `8+3=11` |
| `=` then op | `Add` | continue from result as left operand |
| `50 + 10 %` | `Percent` | context: `a + b%` = `a + a*b/100` → 55; bare `50%` = `0.5` |
| `1/x` `x²` `√x` | unary | acts on current display immediately, `fresh=true` |
| `√(-x)` | `Sqrt` | `Error: Invalid input` |
| `x/0` | `Div` | `Error: Cannot divide by zero`, keypad disabled except `C`/`CE` |
| result `|x| ≥ 1e16` | any | `Overflow` |
| `CE` | `ClearEntry` | display → `0`, keep `acc`/`pending` |
| `C` / `Esc` | `Clear` | full reset, memory + history survive |
| `⌫` | `Backspace` | delete last char; empty → `0` |
| `±` / `F9` | `Negate` | flip sign of display (not of `acc`) |

Display formatting (`fmt.rs`):
- Rust `f64` `Display` is shortest-round-trip → `0.1 + 0.2` renders `0.30000000000000004`.
  **Known v1 deviation** from Windows (which uses arbitrary-precision decimal). Mitigation kept cheap:
  round to 16 significant digits (what Windows shows), then strip trailing zeros → `0.3`.
  Documented, not "fixed" with a decimal crate.
- Group integer part with `,` every 3 digits; never group inside the fraction; plain decimal between
  1e-9 and 1e16, `E` notation outside that.
- `-0` → `0`.

---

## 5. UI spec

Layout, top → bottom (matches Win11 Standard):

```
┌──────────────────────────────────────┐
│ ☰  Standard              ⌄  History  │  top bar
│ MC  MR  M+  M-  MS        (M)        │  memory bar, disabled-state aware
│                    12,345 + 6,789 =  │  expression line (right, dim)
│                             19,134   │  main display (right, large, auto-shrink)
│   %    CE     C     ⌫                │
│  1/x   x²    √x     ÷                │
│   7     8     9     ×                │
│   4     5     6     −                │
│   1     2     3     +                │
│  ±      0     .     =                │
└──────────────────────────────────────┘
```

Implementation notes:
- **eframe 0.37 API** (verified by building): implement `App::ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`
  — `App::update` is gone; `logic()` replaces the old `update` for pre-paint work. Panels take `&mut Ui`
  (`CentralPanel::default().show(ui, ..)`), not `&Context`. Use `use eframe::egui;` — no separate `egui` dep,
  so the two crates can never drift.
- Grid: `ui.available_rect()` → 4 cols × 6 rows, each cell `Rect` from `available / (4,6)`; place buttons with
  `ui.put(rect, Button)` so they scale with the window instead of `egui::Grid` (which does not stretch).
- Min window 320×420, default 360×560, resizable.
- Display font auto-shrinks: pick size from `text.wrap_width / display.len()` buckets.
- Keyboard (handle in `app.rs` before widgets, mark event consumed):
  `0-9` `.` `,` → digit/dot · `+ - * /` → ops · `Enter`/`=` → Equals · `Backspace` → ⌫ ·
  `Esc` → C · `Delete` → CE · `F9` → ± · `%` → percent · `Ctrl+H` → history panel · `Ctrl+D` → clear history.
  Read chars from `ctx.input(|i| i.events())` `Event::Text`, keys from `Key::*`.
- After every input, `ctx.memory_mut(|m| m.request_focus(button_id))`-style trick is **not** needed:
  keep focus on a hidden response area so `Esc`/digits always land. Buttons must not steal keyboard focus
  (`button.interact(focusable=false)` or set `sense` accordingly) — otherwise arrow/Enter behavior breaks.
- Theme (`theme.rs`): dark Fluent-ish — bg `#202020`, digit buttons `#2d2d2d`, operator buttons `#323232`,
  `=` accent `#4cc2ff`-ish on dark, hover/pressed states from egui `Visuals`. Rounded 4px, no borders.
- Accessibility: `button.on_hover_text("seven")` + `ui.ctx().accessibility` on by default; label every button
  with a real string, not a glyph-only symbol, so screen readers work.

---

## 6. Phases

Each phase ends green: `cargo test` + `cargo clippy -- -D warnings` + manual run.

| # | Deliverable | Acceptance |
|---|---|---|
| **0** | `cargo new`, deps pinned, release profile, empty window titled "Calculator" | window opens, 360×560 |
| **1** | `core::Calculator` + `eval` + `fmt`, **no UI** | all §4 table rows pass as `#[test]`; `cargo test` green |
| **2** | Keypad + display wired to core, mouse only | every button clickable, display correct, `2+3=` works visually |
| **3** | Keyboard input + focus handling | type `12+7=` on keyboard without touching mouse; `Esc`/`CE`/`⌫` work |
| **4** | Memory bar + history panel | `MS` enables `MR`, `M+` adds, `Ctrl+H` toggles panel, history capped at 100, click history entry → reuse as input |
| **5** | Error + edge states | ÷0 shows "Cannot divide by zero" and locks keypad; overflow; 16-digit cap; `√-1` |
| **6** | Theme + responsive layout + icon | resize from 320→1200 px wide, no clipped text; dark theme; `.ico` icon |
| **7** | Release build | `cargo build --release --target x86_64-pc-windows-msvc` → single exe, no console window, < 8 MB |

Phase 1 is the honest milestone: the calculator exists and is proven before any pixels.

**Landed:** phase 0 (scaffold, PR-less), phase 1 (engine + parity, PR #1) and phase 2 (keypad + display, PR #2) are on `develop`.

**egui 0.37 API notes** (found the hard way, phase 2 — re-read before touching the UI layer):

- `Context::run` is gone: use `ctx.run_ui(raw_input, |ui| ...) -> FullOutput`, then `.drop_without_applying_deltas()`.
- `RawInput` has `screen_rect: Option<Rect>`, no `pointer` field. Pointer state arrives as
  `Event::PointerMoved` / `Event::PointerButton { pos, button, pressed, modifiers }`.
- `Ui::with_layout` **takes all remaining space** — a display line built with it left the keypad zero pixels tall.
  Use `Ui::allocate_ui_with_layout(size, layout, ..)` to claim an explicit height.
- Headless UI tests work: `Context::default()` + synthetic events, no window, no GPU. Kept as the phase-2
  click proof; it also means "no UI snapshot tests" in §7 costs less than assumed — extend it in phase 3
  for keyboard events rather than reaching for a screenshot harness.

---

## 7. Testing

- **Unit** (`src/core/*` inline `mod tests`): each §4 rule, one test per row.
- **Parity table** (`tests/parity.rs`): `&[(inputs, expected_display)]`, ~40 cases transcribed from the real
  Windows Calculator, e.g. `["2","+","3","=","=","="] → "11"`, `["5","0","+","1","0","%"] → "55"`,
  `["1","/","0","="] → "Error: Cannot divide by zero"`.
- **Property** (cheap, no new dep — hand-rolled loop): display string always parses back to a finite `f64`;
  `Negate(Negate(x)) == x`; no input sequence panics (fuzz digits/ops with `proptest` only if wanted later).
- **Manual checklist**: keyboard-only session, resize at each phase, DPI 125%/150%, memory across many ops.
- No UI snapshot tests in v1 — egui screenshot harness costs more than it returns for 20 buttons.

---

## 8. Risks

| Risk | Mitigation |
|---|---|
| egui 0.37 API drift between minor versions | pin exact `=0.37.0` in `Cargo.toml`; no `*` deps |
| Buttons stealing keyboard focus breaks typing | disable focus on keypad buttons in phase 3, verify with keyboard-only test |
| `f64` display artifacts (`0.30000000000000004`) | 16-sig-digit rounding in `fmt.rs`; documented deviation, decimal crate is the escape hatch |
| Percent/equals-repeat semantics wrong | they are in the §4 spec table + parity tests, not in UI code |
| Text clipping on narrow window | auto-shrink display font + `min_size` on window, checked in phase 6 |
| Windows look "not Fluent" | accept as v1 limitation; egui theming gets close, not pixel-perfect |
| accidental `main` push | **unmitigated** since the ruleset was removed — discipline only; see §9.6 for the cheapest real gate |

---

## 9. Git workflow (gitflow)

### 9.1 Remote

```bash
git init -b main
git remote add origin https://github.com/AgentDudu/rust-calc.git
git remote set-url --push origin https://github.com/AgentDudu/rust-calc.git   # https push; ssh if configured
```

`origin` is the only remote. `target/`, `*.pdb`, `.cargo/` (local) in `.gitignore` from phase 0.

### 9.2 Branch model

| Branch | Lives | Push | PR | Merge |
|---|---|---|---|---|
| `main` | permanent, tagged releases | **never** without your approval | **only after your approval** | **only after your approval** |
| `develop` | permanent, integration | allowed | allowed | allowed (from `feature/*`, `fix/*`, `release/*`) |
| `feature/<slug>` | per work item | allowed | allowed → `develop` | allowed |
| `fix/<slug>` | per bug | allowed | allowed → `develop` | allowed |
| `release/<ver>` | cut from `develop` | allowed | allowed → `main` **draft until approved** | blocked pending you |
| `hotfix/<slug>` | cut from `main` | allowed | allowed → `main` + `develop` | `main` side blocked pending you |

Rules:
1. Branch off `develop` (except `hotfix/*`, off `main`). One work item = one branch = one PR.
2. `git push -u origin <branch>` freely for anything **not** `main`.
3. PRs into `main`: **stop and ask you first.** No push, no merge, no tag on `main` until you say so —
   even when the work looks done. Treat `gh pr merge <n>`, `git push origin main`, `git tag -a` as
   approval-gated commands.
   Approval is **out-of-band (in chat)**: `gh` is authed as `AgentDudu`, the same account that opens the PR,
   and GitHub will not let an author approve their own PR — so no GitHub review can ever be the record here
   unless a second identity is added (§9.6).
4. Never force-push `main` or `develop`. Rebase only your own unpushed feature branch.
5. Keep PRs small: one phase-slice or one feature, not the whole app.

### 9.3 Commit message rules (Conventional Commits)

```
<type>(<scope>): <imperative summary, ≤ 72 chars, no trailing period>

<body: what changed and WHY — wrap at 72>

<footer: BREAKING CHANGE: ..., Refs: #12>
```

- `type` ∈ `feat` `fix` `refactor` `test` `docs` `perf` `style` `chore` `build` `ci`
- `scope` = module: `core` `ui` `fmt` `memory` `history` `theme` `build`
- Imperative mood (`add`, not `added`/`adding`), lowercase summary, no phase/milestone wording —
  **never** `Phase 1:`, `v1:`, `WIP`, `final`, `as planned`.
- Body only when the diff does not explain itself (why, not how). Footer for breaking changes.
- One logical change per commit. Split formatting-only churn from behavior changes.

Examples:
```
feat(core): replace pending operator without evaluating
fix(fmt): trim to 15 significant digits so 0.1+0.2 shows 0.3
test(core): cover equals-repeat and context-sensitive percent
feat(ui): add memory bar with disabled-state awareness
chore(build): pin eframe to 0.37.0 and enable lto
```

Anti-patterns: `Phase 2: keypad`, `feat: work`, `fix bug`, `update`, `feat(ui)!: ...` without footer.

### 9.4 Loop per work item

```bash
git switch -c feature/memory-bar develop
# ...code, cargo test, cargo clippy -- -D warnings...
git add -p && git commit            # conventional message, no phase wording
git push -u origin feature/memory-bar
gh pr create --base develop --fill
gh pr merge --squash --delete-branch   # allowed: target is develop
```

Release cut (needs you before it lands on `main`):
```bash
git switch -c release/0.1.0 develop && git push -u origin release/0.1.0
gh pr create --base main --head release/0.1.0 --title "chore(release): v0.1.0" --draft
# STOP — report PR url, wait for explicit approval, then merge + tag v0.1.0
```

### 9.5 Guardrails

- **No server-side protection on `main`** — ruleset `protect-main` was removed by the owner on purpose
  ("I'll take the risk"). Rule 3 above is now **discipline only**: nothing on GitHub refuses
  `git push origin main`. `main` does not exist on the remote yet, so an accidental push would *create* it.
- Repo default branch: `develop`.
- `.gitattributes` pins `* text=auto eol=lf`.
- `gh` CLI available and authed as `AgentDudu` (https, `repo` scope).
- Commit `Cargo.lock` (binary crate), so CI and your machine build the same tree.

### 9.6 Restoring a real second-party gate (not in place)

Verified facts, so this does not need re-researching:

1. A bot **can** post an approving review: `POST /repos/{owner}/{repo}/pulls/{n}/reviews` with `event: APPROVE`.
   Works with a GitHub App installation token or a fine-grained PAT holding *Pull requests: write*.
   Use REST, not `gh pr review` (the GraphQL `addPullRequestReview` rejects fine-grained PATs).
   `GITHUB_TOKEN` inside Actions **cannot** approve — "GitHub Actions is not permitted to approve pull requests".
2. Ruleset *required reviewers* accepts **teams only**, and the team needs write access to the repo.
   A user-owned repo has no teams, so a hard "must be approved by X" rule needs the repo moved into an org.
3. Cheapest real gate on this personal repo: second account (or GitHub App) + `required_approving_review_count: 1`
   on `main`; the bot posts the approval **only after you say yes in chat**. That makes the merge mechanically
   blocked until a non-author approval exists. Cost: you create the account/app and hand over a token —
   I cannot sign up for a GitHub identity, and scripted signup would breach the ToS.
4. Weaker alternative with no new identity: a required status check `human-approval` that only you trigger
   from the Actions tab. It is a real human action, but I hold `repo` scope, so I could trigger it too.

---

## 10. Start here

```bash
# done: cargo init, eframe pinned, git init + remote + develop pushed (no branch protection on main)
cargo test && cargo clippy --all-targets -- -D warnings
# next: src/core/calc.rs + tests, before any UI
```
