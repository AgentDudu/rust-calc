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

> `Error: ` is the parity table's marker only. The screen shows Windows' wording alone
> (`Cannot divide by zero`), with the failed sum left on the expression line above it.
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
- Keyboard (`ui/keyboard.rs`, polled in `app.rs` before widgets):
  `0-9` `.` `,` → digit/dot · `+ - * /` → ops · `Enter`/`=` → Equals · `Backspace` → ⌫ ·
  `Esc` → C · `Delete` → CE · `F9` → ± · `%` → percent · `Ctrl+H` → history panel · `Ctrl+D` → clear history.
  Read chars from `Event::Text` (layout-independent, handles shifted symbols), the rest from `Key::*`.
  **Map a key on one path only** — `=` on both would fire equals-repeat.
  Events are not consumed: nothing else wants them while v1 has no text input widget. Revisit in phase 4
  if the history panel adds focusable widgets.
- Buttons use `Sense::CLICK`, not `Sense::click()`: the latter is focusable and a focused button swallows Enter.
  Tradeoff — keypad buttons are not Tab-reachable. Windows' Calculator routes keys to the app the same way.
- Title bar shows the display (`"19 - Calculator"`), as Windows does for screen readers. It is also the only
  way to observe the app from outside without pixels, so the keyboard tests use it as an oracle.
- Theme (`theme.rs`): dark Fluent-ish — bg `#202020`, digit buttons `#2d2d2d`, operator buttons `#323232`,
  `=` accent `#4cc2ff`-ish on dark, hover/pressed states from egui `Visuals`. Rounded 4px, no borders.
- Accessibility: `button.on_hover_text("seven")` + `ui.ctx().accessibility` on by default; label every button
  with a real string, not a glyph-only symbol, so screen readers work.

---

## 6. Phases

Each phase ends green: `cargo test` + `cargo clippy -- -D warnings` + manual run. Run them in a tree that
actually recompiled: `cargo clippy` replays cached results, so a warm `target/` can make a broken gate look
clean — which is exactly how `clippy::pedantic` came to be reported clean in phases 5–7 when a clean checkout
reports 26 findings in `src/core/*`. Pedantic stays out of the gate; the findings are a tidy-up branch, not a
release-week change.

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
| **8** | The exe's own icon | Explorer shows the calculator, not Rust's feather (`assets/icon.ico` + `build.rs`) |

Phase 1 is the honest milestone: the calculator exists and is proven before any pixels.

**Landed:** phase 0 (scaffold, PR-less), phase 1 (engine + parity, PR #1), phase 2 (keypad + display, PR #2),
phase 3 (keyboard + focus, PR #3), phase 4 (memory bar + history panel, PR #4), phase 5 (error + edge
states, PR #5), phase 6 (theme + icon, PR #7) and phase 7 (release build, PR #8) are on `develop`.
All seven phases are landed; nothing in the plan is left open except the two items listed as declined
below (the exe file's own icon, and the renderer swap).

**Phase 6 notes.** Default is `ThemePreference::System`, which egui already follows — Windows' calculator
follows the OS theme, so that is parity, and a top-bar button cycles System → Light → Dark for anyone who
wants to force one. Palettes are installed once with `set_visuals_of(Theme, ..)`, so egui swaps them when
the OS flips and no per-frame work is needed.
- **`extreme_bg_color` must equal `panel_fill`, not `window_fill`.** eframe clears the window with it and
  every edge of this app is a panel, so any other value flashes a seam while resizing. A test asserts it
  after it caught the light palette disagreeing with itself.
- **egui 0.37 dims a disabled widget with paint alpha, not a `disabled` palette colour** — there is no
  `Widgets::disabled` field. So a palette cannot be checked for "is greyed still greyed"; only the paint
  list can say, and `a_dead_key_is_painted_dimmer_than_a_live_one` now runs it against both palettes.
- **Icon deviation:** PLAN said `.ico`. The *window* icon is 32×32 RGBA drawn in code — no image crate, no
  binary blob, no anti-aliasing. The *executable file's* icon in Explorer needed a real resource; that is
  phase 8 below.

**Phase 7 notes — the release build, measured not assumed.**

| build | size | CRT imports |
|---|---|---|
| dynamic CRT (as committed before phase 7) | 8,145,408 B = 7.77 MiB | `VCRUNTIME140.dll`, `api-ms-win-crt-*` |
| `-C target-feature=+crt-static` (**shipped**) | 8,366,080 B = 7.98 MiB | none — OS-inbox DLLs only |

- **The flag exists because of an import table, not a hunch.** The dynamic build imports
  `VCRUNTIME140.dll`, which ships with the VC++ redistributable and **not** with Windows: a clean machine
  fails with "VCRUNTIME140.dll not found". A single file that is not actually single. Static costs 220 KB
  and was verified to launch.
- **How each claim was checked, from the binary:** PE header at `e_lfanew`; COFF header is 20 bytes after
  the `PE\0\0` signature (reading `SizeOfOptionalHeader` from the wrong offset yields `0` and a section
  table of zeros); `Subsystem` is at optional-header offset 68 for both PE32 and PE32+ → **2 =
  WINDOWS_GUI, so no console window**, proven from the artifact rather than from `windows_subsystem`.
  Imports come from data-directory entry 1 through the section table → 23 DLLs, no CRT.
  Two offsets bit this twice, so write them down: `SizeOfOptionalHeader` is at `pe+20` (not `pe+8`), and a
  section's `PointerToRawData` is at `section+20` (not `section+16`). Read either wrong and the section table
  decodes to zeros or to plausible-looking garbage — an import list built that way can name DLLs the file does
  not import. The 23-DLL figure is from a walk with both correct, re-run after phase 8 added a resource section.
- **"< 8 MB" is ambiguous and this passes the generous reading.** 7.98 MiB, 8.37 MB decimal. The release
  profile has been maxed since phase 0 (`opt-level="s"`, `lto`, `codegen-units=1`, `panic="abort"`,
  `strip`), so nothing cheap is left.
- **Declined:** eframe's default renderer is **wgpu** (visible as `dxgi.dll` in the imports). Switching to
  `glow` would probably shed megabytes but replaces the render path everything here was tested on.
  `uiautomationcore.dll` is AccessKit — kept, it is how a screen reader reaches the buttons.
- **Not checked:** running on a machine without the VC++ runtime (that is the claim static CRT makes, and
  it was verified by import table, not by a clean VM); `cargo test --release`; the `.pdb` next to the exe
  is a debug artifact, not a runtime dependency.

**Phase 8 notes — the exe's own icon.**

- **What shipped:** `assets/icon.ico` (16/24/32/48 px as 32 bpp BMP + a 256 px PNG entry for Explorer's large
  views, 19 KB) and `build.rs`, which compiles it with the SDK's `rc.exe` and hands the `.res` to the linker.
  Generated, not painted: `python tools/make_icon.py` writes the container, BMP payload and PNG by hand, so the
  asset is reproducible and diffable instead of an opaque blob. No new dependency — `winresource` would still
  have to find the same `rc.exe`.
- **`rc.exe` is not on `PATH`.** It lives in `Windows Kits\10\bin\<version>\x64`, so `build.rs` looks in `$RC`,
  then `PATH`, then the newest SDK directory, and if it finds nothing it warns and builds without an icon.
  A missing icon is not a reason to break someone's build.
- **The two icons had already drifted.** `icon()` still painted `#0078D4` with a white glyph while the palette
  had moved to `#005FB8`, dark band, pale bars. Both now read the same consts, and a test asserts the `.ico`
  contains those three colours — an asset and a drawing have no other way to disagree loudly.
- **`inside_rounded` cut square corners, not round ones.** It tested *every* corner whose corner-box matched,
  and a pixel in the top-left square satisfies all four box tests, so the far corners' circles rejected it: every
  6×6 corner square came off, which reads as a chamfer. Fixed by testing only the corner the pixel is in. The old
  test passed because it probed `(0,0)` (cut either way) and the centre (inside either way) — a test that cannot
  fail is not a test.
- **Verified with the shell's own API:** `Icon::ExtractAssociatedIcon` on the built exe returns 32×32 with band
  `#1B1B1B`, bars `#CFE4FA`, tile `#005FB8`, transparent corner. The `.ico` colour test was mutation-checked —
  blacking the accent out of the file fails it with "tile colour missing".
- **Needs a human:** how the icon reads at 16 px in Explorer and on the taskbar. Its presence in the PE and its
  pixels are proven; legibility at that size is an eye call.

**Release cut — v0.1.0 (done).**

`main` did not exist: the repo was created with `develop` as its default branch, so the cut had to bootstrap it.
`main` = `develop` = `840c63b`, annotated tag `v0.1.0` pushed, `release/0.1.0` kept as provenance (deleting a
remote ref is blocked by the harness safety net). Nothing guards `main` yet — see §9.7 for the gate that would.

**egui 0.37 API notes** (found the hard way, phase 2 — re-read before touching the UI layer):

- `Context::run` is gone: use `ctx.run_ui(raw_input, |ui| ...) -> FullOutput`, then `.drop_without_applying_deltas()`.
- `RawInput` has `screen_rect: Option<Rect>`, no `pointer` field. Pointer state arrives as
  `Event::PointerMoved` / `Event::PointerButton { pos, button, pressed, modifiers }`.
- `Ui::with_layout` **takes all remaining space** — a display line built with it left the keypad zero pixels tall.
  Use `Ui::allocate_ui_with_layout(size, layout, ..)` to claim an explicit height.
- Headless UI tests work: `Context::default()` + synthetic events, no window, no GPU. Kept as the phase-2
  click proof; it also means "no UI snapshot tests" in §7 costs less than assumed — extend it in phase 3
  for keyboard events rather than reaching for a screenshot harness.

**egui 0.37 API notes, phase 4:**

- There is **no `SidePanel` / `TopBottomPanel`** in 0.37 — one `egui::Panel` type covers all four edges:
  `Panel::right("id").exact_size(190.0).resizable(false).show(ui, ..)`, `Panel::top("id").show(ui, ..)`.
  `show` takes `&mut Ui` and there is no `alloc_and_show`.
- Greyed-but-placed widget: `ui.add_enabled_ui(enabled, |ui| ui.put(rect, button))`. `add_enabled` alone
  would drop the button into the cursor flow instead of the rect the layout maths assumes.
- `Button::new((RichText, RichText))` — widgets take tuple atoms (`IntoAtoms`), so one button can hold a
  dim expression and a bold result. Two lines need a layout, not a button.
- A panel shown before the others claims its edge first: the right history panel takes full height and the
  top bar stops at its edge, which is what squeezes the keypad instead of covering it.
- Layout-for-tests pattern (keypad, memory bar, history list): `show` returns the `Rect` it drew into and a
  `#[cfg(test)]` helper maps label/index → click point with the same arithmetic. Any new bar needs both, or
  its clicks are unprovable.
- **Error-state rules (phase 5):** the lockout is drawn, not only enforced — keypad greys everything but
  `C`/`CE`, the memory bar keeps `MC` (MR/M+/M-/MS read a display that is a sentence), history entries grey
  so a past result cannot overwrite the error. The engine ignores the same inputs anyway; the greying is
  for the user. A failed `=` keeps its sum on the expression line (`1 ÷ 0 =` above the message).
- **Anything derived from the display goes after the widgets.** The title bar was computed before the
  panels, and a click is applied *while* they draw: it showed the previous frame's value and, with no
  further input, egui never repainted to correct it.
- Engine note: `C` rebuilds `Calculator` keeping memory and history. A new persistent field must be added
  to that `Calculator { .. }` literal or `C` silently resets it — `memory_set` broke exactly that way.

---

## 7. Testing

- **Unit** (`src/core/*` inline `mod tests`): each §4 rule, one test per row.
- **Parity table** (`tests/parity.rs`): `&[(inputs, expected_display)]`, ~40 cases transcribed from the real
  Windows Calculator, e.g. `["2","+","3","=","=","="] → "11"`, `["5","0","+","1","0","%"] → "55"`,
  `["1","/","0","="] → "Error: Cannot divide by zero"`.
- **Property** (cheap, no new dep — hand-rolled loop): display string always parses back to a finite `f64`;
  `Negate(Negate(x)) == x`; no input sequence panics (fuzz digits/ops with `proptest` only if wanted later).
- **Manual checklist**: keyboard-only session, resize at each phase, DPI 125%/150%, memory across many ops.
  **Run by a human on 2026-10-17, all six items passed:** typed `12+7=` → `19` in the display and in the
  title; `Esc`/`⌫`/`F9` all worked and synced the title; `Ctrl+H` toggled history, `Ctrl+D` cleared it,
  and `Ctrl+H` did *not* type an `h` into the calculator; after `1÷0=` every key but `C`/`CE` looked greyer
  and did not respond; no clipped text at ~320 px or ~1200 px; text readable and buttons unoverlapped at
  scaled DPI. That retires the phase 3 caveat — real OS keystrokes match the `Event::Text`/`Event::Key`
  shapes egui-winit was read to emit, which is the only way that gap could have been closed.
  The history panel's own **Clear** button, which only `Ctrl+D` had exercised, was clicked by hand in the
  phase 6 pass. Phase 6's three eyeball items passed too: the palettes read as Fluent, the **Theme** button
  fits at the 320 px minimum width, and the drawn icon reads as a calculator at taskbar size. So every
  manual item raised so far is closed — the remaining gaps are the ones no eye can see (release-build size,
  and the exe file's own icon, which is not drawn at all).
  Phase 4 adds mouse-only paths proven headlessly (memory buttons, history toggle, entry reuse).
  Phase 5's lockout is proven by clicks being ignored *and* from the paint list:
  `Context::run_ui` returns `FullOutput.shapes`, and `a_dead_key_is_painted_dimmer_than_a_live_one`
  compares the brightness of a locked key's label with a live one. Trap worth keeping: a greyed
  label is the same grey at **half alpha**, and `Color32::to_srgba_unmultiplied` folds that alpha
  back out — the first version measured 645 == 645 and looked like a real bug.
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

`origin` is the only remote. `target/` and `*.pdb` in `.gitignore`. `.cargo/config.toml` **is** committed
since phase 7 — it carries the static-CRT flag every release build needs — after an earlier phase ignored
it as a local file.

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

### 9.6 Restoring a real second-party gate (the bot exists now; the rule does not)

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

### 9.7 The bot account (in place since 2026-07-29)

The second identity exists: `pigeion` / `pigeion-bot[bot]`, a user-owned GitHub App (`01268834a80329e8f532`,
"pi release bot"), installed on this repo with Contents + Pull requests + **Pull request reviews: write**.

| what | value |
|---|---|
| bot token | `gh auth token --hostname github.com.app.pi-release-bot` (keyring service `gh:github.com.app.pi-release-bot`) |
| identity check | `GET /user` → `"bot" in login` |
| approve | REST `POST /issues/{n}/reviews` `event: APPROVE` — the bot is the PR author's sibling identity, so it is not the PR author |
| merge | **still the human account** — GitHub rejects a self-approval by the author, and the bot is not a collaborator |
| token lifetime | installation tokens expire in 1 h; mint with `POST /app/installations/{id}/access_tokens` using a JWT signed with the App's private key (RSA-PKCS1v15-SHA256, doable with `openssl dgst`) |

**What it does and does not buy.** It makes an approval *possible* from a non-author identity, which is what a
required-review rule needs. It does not make the approval mean anything on its own — the gate is only real if
the bot is told to approve after a human says yes. `main` is the branch to put the rule on; `develop` stays
open because every phase merged there.

---

## 10. Start here

```bash
# done: seven phases + the exe icon + v0.1.0 cut to main (main == develop == 840c63b, tag v0.1.0)
cargo test && cargo clippy --all-targets -- -D warnings -D clippy::pedantic
# artifact: target/x86_64-pc-windows-msvc/release/rust-calc.exe — 7.99 MiB, no console, no CRT imports, own icon
# next: nothing in the plan is open. Optional: a GitHub Release with the exe attached, and a branch rule on main
```

---

## 11. Quality gates

Run in a tree that actually recompiled — `cargo clippy` replays cached results, so a warm `target/` can make a
broken gate look clean. Verify lints in a fresh worktree or after `touch`ing the sources.

- `cargo test`: 94 passing (41 core, 49 parity, 4 doc)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: **clean** — 34 findings cleared, see §6
- `cargo doc --no-deps`: no warnings
- `cargo fmt --check`: clean
- Manual: the §7 checklist, every phase.

