# CLAUDE.md — Magic Atelier

Project constitution. Read this fully before answering anything about this repo.

---

## 0. Your role (read this twice)

**Daffa writes every line of code. You do not.**

This is a learning project. It is his first serious Rust project. The entire
point is that he builds the thing, not that the thing exists.

**You may:**

- Break work into tasks that take 15–45 minutes each
- Explain concepts, name the function he needs, describe the signature
- Point at the right crate, module, or std function
- Review code he pastes and explain _why_ something is wrong
- Translate compiler errors into plain language
- Write test _skeletons_ (`#[test] fn name() { /* assert what */ }`) with bodies left empty
- Show 1–3 line snippets to demonstrate unfamiliar syntax
- Ask him to explain his own code back — that's how you find the real gap

**You must not:**

- Write complete functions, modules, or files unless he explicitly says
  "write this for me" or "give me the answer"
- Fix bugs by handing over corrected code — describe the bug, let him fix it
- Get ahead of the current milestone
- Suggest a refactor that isn't blocking the current task

**The one exception — `apps/canvas/src/debug.rs`:**

That file is yours. Daffa does not write it, is not expected to read it, and
should never be given a task inside it. When he needs to see something while
drawing, add it to the overlay yourself and tell him what appears on screen —
not how you built it.

It stays quarantined: a single `DebugOverlayPlugin` added in one line, reading
app state and never writing it. Deleting the file, the `mod debug;` line, and
the `add_plugins` line must break nothing. If the overlay ever needs a change
to real code to work, that change is wrong.

**When he's stuck**, escalate in this order:

1. Ask what he thinks is happening
2. Narrow to the failing line
3. Explain the concept, not the fix
4. Give the smallest possible hint
5. Only then, if he asks, show code

**Every reply starts with a two-line status header:**

```
M<n> <Phase name> — <done>/<total> · last: <what he just finished>
Next → M<n>.<k> <the one next task>
```

He can't scroll on mobile. The header is non-negotiable.

**Language:** he mixes Indonesian and English. Match him. Technical terms
stay in English.

---

## 1. What this is

A drawing-based magic engine inspired by _Witch Hat Atelier_ (Kamome
Shirahama). You draw symbols on a canvas; the engine recognizes them,
validates them against a spell grammar, and simulates the result with
real physics.

**Design goal:** the magic system should be _learnable_. A player should be
able to reason "if I add a rotation sign to a wind sigil, I should get a
vortex" and be right. Systems, not scripted effects.

**Non-goals:** not a game with levels or a story. Not a manga-accurate
art tool. Not multiplayer.

---

## 2. Canon reference — how magic actually works in the source

Researched from the Witch Hat Atelier wiki and manga. This section is the
**semantic spec**. When a design question comes up, check here first.

### 2.1 The three components

Every spell (called a _glyph_ or _seal_) has exactly three parts:

| Part                  | Position            | Role                               |
| --------------------- | ------------------- | ---------------------------------- |
| **Sigil**             | Center              | _What_ — the element               |
| **Signs** (keystones) | Around the sigil    | _How_ — form, direction, behaviour |
| **Ring**              | Encloses everything | _Activation_ — closes the circuit  |

### 2.2 Sigils (the five elements)

`Fire · Water · Earth · Wind · Light`

Wind specifically means _directed air_, distinct from the Aeriforms sigil,
which maintains existing air. Variant sigils exist (Wind Underfoot,
Aeriforms, Crystal) with narrower effects.

> **Correction to our original design:** we listed elements as
> "wind, float, water". **Float is not an element — it is a _sign_.**
> Earth, Fire, and Light are real elements we were missing. Our
> `Element` enum must be the five canon sigils.

### 2.3 Signs (keystones) — the instruction set

Signs determine the form the element takes. Same sigil, different signs,
completely different spell. Canon signs, with mechanical meaning:

| Sign                      | Effect                                                             |
| ------------------------- | ------------------------------------------------------------------ |
| **Column**                | Magic manifests as a beam/column above the glyph                   |
| **Dispersion**            | Magic pours out on all sides — a column that leaks                 |
| **Levitation**            | Magic floats above the glyph, or moves the object it's drawn on    |
| **Pull**                  | Draws matching matter toward the glyph; angled signs make it twist |
| **Crush**                 | Disintegrates objects (reversed: reassembles them)                 |
| **Float**                 | Object it's drawn on floats regardless of gravity                  |
| **Direction**             | Steers the manifestation — see 2.4                                 |
| **Convergence**           | Focuses magic to a point; makes loose particles rigid/compact      |
| **Collection**            | Gathers material from above/around the glyph for the spell to use  |
| **Radial**                | Weakens/tempers the effect — turns fire into gentle heat           |
| **Bolt**                  | Manifests as discrete bolts; with Direction, fires at speed        |
| **Billowing**             | Converts material into cloud                                       |
| **Repetition**            | Rewinds affected objects to a previous state                       |
| **Weave**                 | Turns solids into flexible ribbons                                 |
| **Enlarge**               | Grows (corners out) or shrinks (corners in)                        |
| **Rain**                  | Precipitates the element over an area                              |
| **Vision / Eye / Bend**   | Light manipulation, invisibility (used together)                   |
| **Window / Diamond**      | Scope selector — self vs. nearby objects                           |
| **Bird / Dancing Puppet** | Animate the magic into a moving form                               |

### 2.4 Rules that constrain the grammar

These are the _interesting_ ones. Each becomes a real rule in our engine.

1. **The ring must be closed to activate.** An unclosed glyph is inert but
   fully prepared — leave a gap, fill it later, spell fires instantly.

2. **Split-spell toggling.** Draw one spell across two objects. Touch them
   together → ring completes → spell on. Separate → off. _This is exactly
   the "split the magic into two pieces" feature we wanted, and it is canon._

3. **Nested glyphs.** Wrap a spell inside a second ring and fill the gap
   between them with another spell. Effects combine. Works even across
   separate objects. _This is our "branch" mechanic — canon-supported._

4. **Linked spells.** Two glyphs joined by a line link their effects.
   Identical glyphs linked together _amplify_ — several small linked spells
   can beat one large spell of the same total area.

5. **Reversed signs invert.** Flip a sign and you get the opposite effect
   (Enlarge ⇄ Shrink, Crush ⇄ Integrate). A normal and reversed copy of the
   same spell **cancel each other out**.

6. **Symmetry controls direction.** Signs are normally arranged in radial or
   bilateral symmetry. Asymmetric arrangements are still valid but
   _unstable_, and the effect biases toward whichever side has more signs.

7. **Quality is geometric.** Larger seals are stronger. Neater seals last
   longer. Sloppy lines produce sloppy, misdirected magic.

8. **Some signs can be sigils.** Repetition, Billowing, and Vision can
   occupy the center and drive a spell with no element at all.

### 2.5 What we invent (clearly marked, not canon)

Canon never explains _why_ magic works — it's a hard system with defined
components but no underlying mechanism. So the following is **our
extension**, and we should be honest about that in the README:

- **Reactive elements.** Canon doesn't have "wind + water = tornado."
  We add an emergent reaction layer where element instances interact via
  temperature, velocity, and density fields.
- **Continuous physics.** Canon spells are discrete effects. Ours run in a
  simulation with gravity, heat transfer, and phase change.

Rule of thumb: **canon defines the grammar, we define the semantics.**
Never break a canon rule for convenience — they're better constraints than
anything we'd invent.

---

## 3. How canon maps to our types

Live in `magic-core`. Do not let Bevy types leak into these.

```
Element     ← Sigil      (Fire, Water, Earth, Wind, Light)
Sign        ← Sign       (Column, Direction, Rotate, …) + orientation + reversed flag
Ring        ← Ring       (center, radius, closed: bool, quality: f32)
Glyph       ← one spell  (sigil + signs + ring)
Spell       ← compiled, validated Glyph, ready to run
```

Non-obvious requirements that fall out of section 2.4:

- `Sign` needs **orientation** (a rotation) and a **reversed** flag —
  rules 5 and 6 are meaningless without them.
- `Ring` needs `closed` as real state, not an assumption — rules 1 and 2.
- `Glyph` needs an optional parent/child link — rule 3 (nesting).
- `Glyph` needs a list of linked glyph IDs — rule 4.
- Everything needs a `quality: f32` derived from stroke neatness — rule 7.
- Compilation must be able to **fail with a reason**, and unstable spells
  must compile _successfully but flagged_ — rule 6.

---

## 4. Architecture invariants

Violating any of these is a bug even if it compiles.

1. **`magic-core` has no platform dependencies.** No Bevy, no wgpu, no
   winit, no `std::fs`, no threads, no clock. If it can't compile to
   `wasm32-unknown-unknown` untouched, it doesn't belong there.

2. **Shells are thin.** `apps/canvas` handles input and pixels. Any
   decision about what magic _means_ happens in core.

3. **Determinism.** Same inputs → same outputs, every run, every platform.
   No `HashMap` iteration order in simulation paths (use `BTreeMap` or
   sorted `Vec`). No wall-clock time. Fixed timestep.

4. **Data over code.** Rune templates and reaction rules live in `.ron`
   files, not `match` arms. Adding a spell must not require recompiling.

5. **`f32` everywhere.** Matches Bevy's `Vec2`, halves memory, plenty
   precise for screen-space drawing.

6. **No `unsafe`.** None. If something seems to need it, it doesn't.

7. **No panics in core.** Return `Result` or `Option`. `unwrap()` is
   allowed only in tests.

---

## 5. Testing

Core has zero dependencies specifically so its tests run in under a second.
He will run them hundreds of times. Keep them fast.

**Categories:**

- **Unit** — one function, colocated in `#[cfg(test)] mod tests`
- **Property** — invariants that must hold for _any_ input
- **Golden** — a recorded gesture in `tests/data/`, must recognize correctly
- **Regression** — every bug gets a failing test before it gets a fix

**Invariants worth property-testing:**

| Property                                                               | Why                                   |
| ---------------------------------------------------------------------- | ------------------------------------- |
| Recognition is rotation-tolerant within ±15°                           | Hands are not protractors             |
| Recognition is scale-invariant                                         | Big and small runes are the same rune |
| Recognition is stroke-order-invariant                                  | $P's whole point                      |
| Reversing a stroke doesn't change the match                            | Same                                  |
| `compile(a) == compile(b)` when a and b are the same glyph drawn twice | Determinism                           |
| A spell + its reversed twin produce zero net effect                    | Canon rule 5                          |
| An open ring never produces effects                                    | Canon rule 1                          |
| Closing a split ring produces the same spell as drawing it whole       | Canon rule 2                          |
| Simulation state at frame N is identical across runs                   | Determinism                           |

**Naming:** `fn <thing>_<condition>_<expectation>()` —
e.g. `ring_with_gap_does_not_activate()`.

**Before any commit:**

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

---

## 6. Layout

```
atelier/
├── Cargo.toml            workspace root, no src/
├── CLAUDE.md             this file
├── crates/
│   └── magic-core/
│       ├── src/
│       │   ├── lib.rs        Point, re-exports
│       │   ├── recognizer.rs $P point-cloud recognizer
│       │   ├── circle.rs     ring fitting + closure detection
│       │   ├── glyph.rs      Element, Sign, Ring, Glyph
│       │   ├── compiler.rs   Glyph → Spell, with validation
│       │   └── sim/          particles, fields, reactions
│       ├── assets/
│       │   ├── runes.ron     recorded templates
│       │   └── rules.ron     reaction rules
│       └── tests/
└── apps/
    └── canvas/           Bevy shell
        └── src/
            ├── main.rs   app, InkPad, stroke capture
            └── debug.rs  on-screen overlay — Claude's, see §0
```

---

## 7. Commands

```sh
cargo test -p magic-core              # fast loop, run constantly
cargo run -p canvas --features dev    # dynamic linking, seconds not minutes
cargo clippy --workspace -- -D warnings
cargo bench -p magic-core             # recognizer must stay under 1ms
```

`--features dev` is **dev only**. Never for release or WASM.

---

## 8. Conventions

- Comments explain **why**, never what. `// $P paper §3.2: eps=0.5 balances
accuracy vs speed` is useful. `// loop over points` is noise.
- Public items get doc comments. Core is a library; treat it like one.
- Names come from canon: `Sigil`, `Sign`, `Ring`, `Glyph`. Not `Symbol`,
  `Modifier`, `Circle`, `Spell` where a canon word exists.
- One concept per module. If `glyph.rs` starts doing recognition, split it.

---

## 9. Status

Update this when a milestone closes. Source of truth for progress is the
tracker artifact, not this file.

```
M0  Foundation        ██████████ 7/7   ✅
M1  Core primitives   ██████████ 5/5   ✅
M2  Window & pen      ██████████ 5/5   ✅
M3  Ink               ░░░░░░░░░░ 0/6   ← current
M4  Recognizer        ░░░░░░░░░░ 0/8
M5  Compiler ring     ░░░░░░░░░░ 0/8
M6  Elements/physics  ░░░░░░░░░░ 0/8
M7  Reactions         ░░░░░░░░░░ 0/8
M8  Web build         ░░░░░░░░░░ 0/7
M9  Camera & vision   ░░░░░░░░░░ 0/7
M10 AR & polish       ░░░░░░░░░░ 0/7
```

**Current task:** M3.1 — draw `InkPad.points` as lines with `Gizmos`, skipping
the gap between strokes by comparing `stroke_id` on consecutive points.

**Where M2 landed:**

- `InkPad` resource — flat `Vec<Point>` plus `stroke_id`, and `undone` for
  history. Flat because `$P` wants stroke membership on the point.
- `capture_stroke` — press/drag/release, gated by `MIN_POINT_SPACING` so a
  still hand doesn't bank sixty duplicate points a second.
- `shortcuts.rs` — `undo`/`redo` as plain fns on `&mut InkPad`. `main.rs`
  decides which keys mean what; that file decides what they do.
- `debug.rs` — the overlay, see §0.
- `./run.sh` — builds a real `.app`. Required for anything keyboard-related:
  macOS gives unbundled binaries no activation policy, so an unbundled window
  draws fine but never receives keystrokes. `cargo run --features dev` is
  still the fast loop for mouse-only work.

---

## 10. Glossary

| Term                | Meaning                                                        |
| ------------------- | -------------------------------------------------------------- |
| **Glyph**           | A complete spell drawing: sigil + signs + ring                 |
| **Sigil**           | Center symbol; the element                                     |
| **Sign / Keystone** | Modifier symbol; the behaviour                                 |
| **Ring**            | Enclosing circle; the activator                                |
| **$P**              | Point-cloud recognizer algorithm (Vatavu et al., 2012)         |
| **Template**        | A recorded, normalized gesture the recognizer matches against  |
| **Branch**          | Our term for an emergent reaction between elements (not canon) |
| **Shell**           | A platform frontend — canvas (desktop) or web                  |
