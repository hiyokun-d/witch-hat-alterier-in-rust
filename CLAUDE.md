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

Source of truth is the Independent Witch Hat Atelier Wiki
(`witchhatatelier.telepedia.net`), pages **Magic**, **Sigils_Explained**, and
**Signs_Explained**. This section is the **semantic spec**. When a design
question comes up, check here first.

Where the wiki says an effect is unknown, this section says unknown. Do not
fill those gaps with invention — an honest hole is a better spec than a
plausible guess, and §2.5 is where our own ideas are allowed to live.

### 2.1 The three components

Every spell (called a _glyph_ or _seal_) has exactly three parts:

| Part                  | Position            | Role                               |
| --------------------- | ------------------- | ---------------------------------- |
| **Sigil**             | Center              | _What_ — the element               |
| **Signs** (keystones) | Around the sigil    | _How_ — form, direction, behaviour |
| **Ring**              | Encloses everything | _Activation_ — closes the circuit  |

**The sigil is optional.** Most spells have one, but Repetition, Billow, and
Vision can occupy the center and drive a spell alone. A glyph with no sigil is
a legal spell, and the compiler must accept it.

### 2.2 Sigils — families and variants

Sigils are not a flat list of five elements. They are **families**, each
holding variants whose behaviour differs from one another. Behaviour belongs to
the variant, never to the family.

**Fire** — flame, heat, light

| Variant              | Effect                                                                                            |
| -------------------- | ------------------------------------------------------------------------------------------------- |
| **Fire**             | Creates and manipulates flame or heat                                                             |
| **Unburning Flames** | Heatless flame — the phantasmal fireball. Exact mechanism unknown; may need supporting signs      |
| **Light**            | Manifests magic as light. A fire variant, classified separately only because light spells are many |

**Water**

| Variant   | Effect                                                                                                                                  |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Water** | Manipulates, collects, and creates water. Long-duration water spells _collect_ rather than create, implying creation costs more energy |

**Earth**

| Variant   | Effect                                                                     |
| --------- | ---------------------------------------------------------------------------- |
| **Earth** | Manipulates wood, stone, sand, soil. **Cannot create them** |

**Air**

| Variant            | Effect                                                                          |
| ------------------ | --------------------------------------------------------------------------------- |
| **Wind**           | Moves and manipulates air. **Cannot create it**                                 |
| **Aeriforms**      | Creates and manipulates air. **Cannot move it**                                 |
| **Wind Underfoot** | Supports solid objects suspended in air — an air platform. Mechanism unclear    |
| **Whorling Winds** | Manipulates air through rotation. Looks visually unlike the other air sigils, reason unknown |

**Time**

| Variant        | Effect                                                                                                                             |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Repetition** | Continuously resets affected objects to the state they held when the spell took hold. Spring-like: makes soft things elastic, stops rot |
| **Stop**       | Halts time outright for affected objects. Paired with another sigil it stops one aspect only — with fire, heat stops changing        |

**Misc**

| Variant      | Effect                                                                            |
| ------------ | ----------------------------------------------------------------------------------- |
| **Crystal**  | Creates and manipulates crystalline objects. Only Richeh uses it                   |
| **Guidance** | Attracts objects matching parameters set by the other signs and sigils in the spell |

> **Critical rule — a sigil's size and location within a seal do not change its
> behaviour** (with rare exceptions). The recognizer must therefore score sigils
> on **shape only**, never on position or scale.

> **Corrections to our older design.** Float was never an element — it is a
> sign. Rotation was never a sign — Whorling Winds is a sigil, so rotation is a
> property of the element, not a modifier. Both are fixed below and in §3.

### 2.3 Signs (keystones) — the instruction set

Signs control the **form** the sigil's effect takes, plus its size and
direction. Same sigil, different sign, completely different spell: water +
dispersion pours out on all sides like an overflowing bucket, water + column
shoots out fast like a hose pinched by a finger.

The wiki sorts signs into three **provenance tiers**, and we keep that
distinction in the data model — see the `canon` field in §3. It records which
behaviours are established and which are fan reconstruction, so a bug report
about a wrong effect can be answered with "that was never canon."

#### Officially named

| Sign            | Effect                                                                                                                                                          |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Column**      | Manifests as a column or beam above the glyph. Unbalanced signs make it manifest toward whichever side has more (radial symmetry does not count as unbalanced). Shorter line usually faces outward |
| **Dispersion**  | A column that leaks in all directions instead of beaming                                                                                                        |
| **Levitation**  | Magic floats above the glyph, or moves the object it is drawn on. Movement follows where the signs point; arrow side usually faces inward                        |
| **Pull**        | Pulls matter of the same kind as its magic toward the glyph when arrows point inward. Angled signs make the pull twist. Inverted likely pushes                   |
| **Crush**       | Disintegrates objects, or reassembles them when reversed. Only ever seen with earth; other pairings unknown                                                     |
| **Float**       | Makes the object it is drawn on float regardless of gravity. Partly retconned into levitation; surviving uses are float-only                                     |
| **Region**      | Determines _where_ magic manifests relative to the glyph — four cases, see below                                                                                |
| **Convergence** | Focuses magic to a single point; makes loose particles rigid and compact (sand becomes hard — serpent's bed of sand). One triangle point usually faces inward     |
| **Collection**  | Collects material, possibly magic, from above and around the glyph for the spell to use. Open side faces inward                                                  |
| **Billow**      | Converts material into cloud. Needs collection to gather the material first; some materials cannot convert. **Can take the place of a sigil**                    |
| **Repetition**  | Resets affected objects to a previous state. **Can take the place of a sigil**                                                                                  |
| **Weave**       | Turns solid objects into long flexible ribbons on contact. Invented by Richeh. Must surround the central sigil                                                   |

**Region's four cases.** Computed from where _all_ region signs point
collectively, not from any one sign:

| Arrangement                    | Where the magic manifests                            |
| ------------------------------ | ---------------------------------------------------- |
| All pointing one side          | Shoots that direction                                |
| All pointing inward            | Only inside the ring                                 |
| All pointing outward           | Only outside the ring — no effect inside             |
| Opposed (some in, some out)    | Only _on_ the ring itself (floating drops)           |

**Documented, effect not yet described.** Real names, no known behaviour. Model
them; do not invent what they do.

`Cool · Strengthen · Sights Set · Entwine · Sign of Wind · Aeriforms Defined · Glaives`

#### Unofficially named

Fan-reconstructed by comparing spells. Plausible, not established.

| Sign          | Effect                                                                                                                                              |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Window**    | Restricts the spell to affecting only the object it is drawn on                                                                                     |
| **Diamond**   | Restricts the spell to affecting only nearby objects, not the object it is drawn on                                                                 |
| **Enlarge**   | Grows objects (corners out) or shrinks them (corners in). Window vs Diamond decides self vs nearby. Goes in the centre                               |
| **Crosshair** | From rainflinger. Three candidate functions: erase things of the same magic aspect, restrict manifestation to inside matching objects, or define an area of effect |
| **Radial**    | From snugstone. Likely weakens the spell — turns fire into gentle heat                                                                              |
| **Bolt**      | Manifests magic as bolt-like projectiles. With region to aim it, fires at dangerous speed                                                            |
| **Eye**       | Appears with vision in gathering shadows and the makeover mask. Related to illusion and light manipulation. Never seen alone, so its solo effect is unknown |
| **Vision**    | Relates to sight and visual perception. With eye, creates illusions; without eye (Qifrey's glasses), aids sight. **Can take the place of a sigil**   |
| **Bend**      | Appears in gathering shadows, petrification, and possibly wall bend. Common theme is bending or altering reality — vision, physical objects, or reality itself |
| **Rain**      | Produces the sigil's magic as rainfall over the immediate area. Surrounds the central sigil                                                          |
| **Puppet**    | Lets a user control the movement of the object the spell is drawn on, apparently by mind. Movement type depends on the sigil — wind puppet spells only move through air |

Named, effects not yet documented: `Bind · Orb · Link`

#### Decorative

No mechanical effect. Model them, give them zero behaviour.

| Sign             | Effect                                                                                                       |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| **Bird**         | Projects a bird made of the glyph's magic that flies around. Sigil goes in its centre                         |
| **Animal Signs** | Zozah Peninsula animal shapes. No practical utility; use is declining, hobby and decoration only              |

### 2.4 Ring and structural rules

These are the _interesting_ ones. Each becomes a real rule in our engine.

1. **Everything must be inside the ring, or connecting to it.** Every sigil and
   sign must be drawn inside the ring or touching it; anything else does not
   count toward the spell. Note the _connecting to_ clause — containment cannot
   be a pure point-in-circle test.

2. **A spell activates only when its ring is complete.** Leaving a gap prepares
   a spell to be fired later by closing it.

3. **Spell toggling.** Draw one spell in two parts across two objects. Touching
   them completes the ring and turns it on; separating turns it off (the
   glowstone path).

4. **Nesting.** Wrap a spell in a second ring and fill the gap between them
   with another spell to combine both effects — even across separate objects.
   Whether nested spells activate simultaneously or only when the outermost
   ring closes is **unclear in canon**. Model the ambiguity explicitly rather
   than picking one.

5. **Linked spells.** Two glyphs joined by a line link their effects. Identical
   or similar linked glyphs amplify each other — several small linked copies
   can beat one large spell of the same total area.

6. **Reversed signs invert their effect.** A normal spell and its reversed twin
   cancel out completely.

7. **Symmetry.** Signs are usually arranged in radial or bilateral symmetry.
   Asymmetric spells are perfectly valid but sometimes unstable.

8. **Quality is geometric.** Larger seals are stronger. Neater seals last
   longer.

### 2.5 What we invent (clearly marked, not canon)

Canon never explains _why_ magic works — it's a hard system with defined
components but no underlying mechanism. So the following is **our extension**,
and we should be honest about that in the README:

- **Reactive elements.** Canon doesn't have "wind + water = tornado." We add an
  emergent reaction layer where element instances interact via temperature,
  velocity, and density fields.
- **Continuous physics.** Canon spells are discrete effects. Ours run in a
  simulation with gravity, heat transfer, and phase change.

Two things that are _not_ on this list, though we once thought they were:

- **Rotation is canon.** Whorling Winds manipulates air through rotation. We do
  not need to invent a rotate modifier, and we must not — rotation lives on the
  sigil.
- **The capability model in §3 is derived, not invented.** `can_create` /
  `can_move` fall straight out of the wiki's own wording: wind moves air but
  cannot create it, aeriforms creates air but cannot move it, earth manipulates
  but never creates. We are reading canon, not extending it.

Rule of thumb: **canon defines the grammar, we define the semantics.** Never
break a canon rule for convenience — they're better constraints than anything
we'd invent.

---

## 3. How canon maps to our types

Live in `magic-core`. Do not let Bevy types leak into these.

```
Sigil       ← one sigil variant + its family + its capabilities
Sign        ← Sign (Column, Region, …) + orientation + reversed + canon tier
Ring        ← Ring   (center, radius, closed: bool, quality: f32)
Glyph       ← one spell (optional sigil + signs + ring + nesting + links)
Spell       ← compiled Glyph + warnings, ready to run
```

### 3.1 Sigils are two levels, not one

`Element` as a flat five-variant enum is wrong and has to go. A sigil is a
**family** plus a **variant**, and every behavioural question is answered by
the variant. The family exists for grouping and for reactions, nothing else.

- `SigilFamily` — `Fire · Water · Earth · Air · Time · Misc`
- `Sigil` — the variant: `Fire, UnburningFlames, Light, Water, Earth, Wind,
  Aeriforms, WindUnderfoot, WhorlingWinds, Repetition, Stop, Crystal, Guidance`

Since a sigil's size and position are canonically irrelevant (§2.2), neither
belongs on this type, and the recognizer must not feed them in.

### 3.2 Capabilities — the most valuable thing in the research

Each sigil variant carries what it is _able to do_:

```
can_create · can_manipulate · can_move · can_collect
```

This is not flavour text. Wind moves air but cannot create it; aeriforms
creates air but cannot move it; earth manipulates wood and stone but never
creates them. Encoding that gives the simulation **real conservation rules**
instead of spawning matter from nothing — a wind spell in a sealed room has to
find its air somewhere. Every emergent behaviour worth having comes from this
constraint, so it goes in early, not as polish.

### 3.3 What each type needs

- **`Sign`** needs `orientation` and a `reversed` flag — rules 6 and 7 are
  meaningless without them. It also needs a `canon` tier
  (`Official | Unofficial | Decorative`) and a `can_be_sigil` answer, true for
  Repetition, Billow, and Vision.
- **Region analysis** is a function over the whole sign set, not a per-sign
  property. The four cases in §2.3 are computed from where every region sign
  points collectively.
- **`Ring`** needs `closed` as real state, not an assumption — rules 2 and 3.
  Its containment test must satisfy rule 1's _connecting to_ clause: a sign
  touching the ring counts, so a point-in-circle check is insufficient.
- **`Glyph`** needs an optional nesting parent (rule 4), a list of linked glyph
  ids (rule 5), a symmetry classification
  (`Radial | Bilateral | Asymmetric`, rule 7), and a stability flag that
  asymmetry sets **without** causing failure.
- **`Glyph.sigil` is `Option`** — §2.1. A sigil-less glyph driven by Repetition,
  Billow, or Vision compiles fine.
- **Quality** is an `f32` derived from stroke neatness, on everything — rule 8.
- **Compilation returns a spell _plus warnings_.** Unstable is not an error.
  Nesting activation order is canonically undecided (rule 4), so that ambiguity
  is represented in the type, not resolved by a coin flip.

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
| A spell + its reversed twin produce zero net effect                    | Canon rule 6                          |
| An open ring never produces effects                                    | Canon rule 2                          |
| Closing a split ring produces the same spell as drawing it whole       | Canon rule 3                          |
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
| **Sigil**           | Center symbol; the element. Optional — some signs can take its place |
| **Sign / Keystone** | Modifier symbol; the behaviour                                 |
| **Ring**            | Enclosing circle; the activator                                |
| **Region**          | The sign that decides _where_ magic manifests. Formerly miscalled "Direction" |
| **Whorling Winds**  | Air sigil that works through rotation — why we have no rotate sign |
| **Canon tier**      | A sign's provenance: Official, Unofficial (fan-reconstructed), or Decorative |
| **$P**              | Point-cloud recognizer algorithm (Vatavu et al., 2012)         |
| **Template**        | A recorded, normalized gesture the recognizer matches against  |
| **Branch**          | Our term for an emergent reaction between elements (not canon) |
| **Shell**           | A platform frontend — canvas (desktop) or web                  |
