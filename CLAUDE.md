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

**Nothing above constrains drawing _order_.** Canon never says the ring comes
first, and rules 2 and 3 both describe seals whose contents exist before the
ring closes — an unclosed ring is a fully prepared spell waiting on its last
stroke, and half a split seal is drawn with no complete ring at all. A player
who draws the sigil, then the signs, then the ring last is doing the canonical
thing, not a weird thing.

So the engine never enforces an order, and never rejects a stroke for arriving
too early. It also never _forbids_ ring-first — both orders, and every order in
between, produce the same glyph. See §3.3 for what that requires of the types.

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
SigilId     ← names a sigil in sigils.ron
SignId      ← names a sign in signs.ron
Sign        ← SignId + placement + orientation + reversed
Ring        ← center, radius, closed: bool, quality: f32
Glyph       ← one spell (optional sigil + signs + ring + nesting + links)
Catalog     ← the .ron files, parsed and cross-checked
Spell       ← compiled Glyph + warnings, ready to run
```

### 3.1 Nothing about a sigil is an enum

`Element` as a flat five-variant enum is wrong and has to go — but so is the
thirteen-variant enum that first replaced it. §4.4 says adding a spell must not
require recompiling, and an enum of sigils breaks that the day someone adds one.

So the split is **schema in code, content in data**:

- **Code** holds the closed vocabularies the data draws from — `Family`,
  `Tier`, `Confidence`, `Class`, `Slot`, `Scope`, `RegionPattern`. Adding a
  seventh family is a genuine design change and should cost a recompile.
- **Data** holds every sigil, sign, and spell. `SigilId` and `SignId` are
  newtypes over `String`, and `Catalog` is the only thing that maps an id to
  behaviour.

Since a sigil's size and position are canonically irrelevant (§2.2), neither
belongs on the type, and the recognizer must not feed them in.

### 3.2 Capabilities — the most valuable thing in the research

Each sigil variant records what it is _able to do_, in `sigils.ron`:

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
  meaningless without them — plus a **`placement`**: the angle it sits at around
  the ring. Inward and outward are questions about direction _relative to
  position_, so the same sign at the top and at the bottom of a ring means
  different things. Rule 1's containment test needs the position too.
- **Tier and substitution live in the data**, not on the type. `Catalog` answers
  whether a sign is decorative and whether it can stand in for a sigil.
- **Region analysis** is a function over the whole sign set, not a per-sign
  property. The four cases in §2.3 are computed from where every region sign
  points collectively, and the result reuses `RegionPattern` so a computed
  arrangement can be compared straight against a spell fixture.
- **`Ring`** needs `closed` as real state, not an assumption — rules 2 and 3.
  Its containment test must satisfy rule 1's _connecting to_ clause: a sign
  touching the ring counts, so a point-in-circle check is insufficient.
- **`Glyph`** needs an optional nesting parent (rule 4), a list of linked glyph
  ids (rule 5), a symmetry classification
  (`Radial | Bilateral | Asymmetric`, rule 7), and a stability flag that
  asymmetry sets **without** causing failure.
- **`Glyph.sigil` is `Option`** — §2.1. A sigil-less glyph driven by repetition,
  billow, or vision compiles fine.
- **Glyph assembly is geometric, never chronological.** Grouping strokes into a
  glyph is a query over the finished pad — find the closed loops, take
  everything each one contains or touches (rule 1), classify the rest. Stroke
  order is not an input, which is the same reason the recognizer uses $P.
  Concretely: `Glyph::new` must stop demanding a ring at construction, since
  that signature alone makes ring-first the only expressible order.
- **Strokes belonging to no ring are inert, not invalid.** They stay on the pad
  unclassified. A player halfway through a seal has drawn nothing wrong.
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

- **Unit** — one function. The module keeps a three-line
  `#[cfg(test)] #[path = "tests/<module>.rs"] mod tests;` and the tests live in
  `src/tests/<module>.rs`. Still a **child** of the module it tests, so private
  items stay reachable — a crate-root `tests/` directory would be an
  integration target, could only see the public API, and could not test
  `apps/canvas` at all, since a binary crate has nothing to link against
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
| The same seal compiles identically drawn ring-first and ring-last      | §2.4 — order is never an input         |
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
│       │   ├── glyph.rs      Sign, Ring, Glyph — identity, no behaviour
│       │   ├── catalog.rs    .ron loader; the only thing knowing behaviour
│       │   ├── arrangement.rs symmetry + region analysis over a sign set
│       │   ├── assembly.rs   strokes → rings; closure by endpoints
│       │   ├── stroke.rs     path length + even resampling
│       │   ├── templates.rs  recorded gestures, loaded from .ron
│       │   ├── compiler.rs   Glyph → Spell, with validation
│       │   ├── sim/          particles, fields, reactions
│       │   └── tests/        one file per module above — see §5
│       └── the-magic-assets/
│           ├── sigils.ron    13 sigils + capabilities
│           ├── signs.ron     35 signs, three canon tiers
│           └── spells.ron    13 spell fixtures + 7 edge cases
└── apps/
    └── canvas/           Bevy shell
        └── src/
            ├── main.rs      app, InkPad, capture, paper, ink
            ├── shortcuts.rs undo / redo / clear on &mut InkPad
            ├── debug.rs     on-screen overlay — Claude's, see §0
            └── tests/       one file per module above
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
M1R Canon rework      ██████████ 7/7   ✅
M2  Window & pen      ██████████ 5/5   ✅
M3  Ink               ██████████ 7/7   ✅
M4  Recognizer        ██████████ 9/9   ✅
M5  Compiler ring     ░░░░░░░░░░ 0/8   ← current
M6  Elements/physics  ░░░░░░░░░░ 0/8
M7  Reactions         ░░░░░░░░░░ 0/8
M8  Web build         ░░░░░░░░░░ 0/7
M9  Camera & vision   ░░░░░░░░░░ 0/7
M10 AR & polish       ░░░░░░░░░░ 0/7
```

M4 came in at nine tasks, not the eight first planned: M4.2b — assembling
strokes into rings — was scoped as part of M4.2 and turned out to be its own
piece of work.

**Current task:** M5.1 — compile a `Glyph` from the rings and contents the
recognizer now produces. Blocked on nothing in code; blocked in practice on
`templates.ron`, which is empty until the rune shapes are traced.

**Where M4.1 landed:**

- `circle.rs` — Taubin's fit, not Kåsa's. Kåsa is biased on arcs, and arcs are
  not an edge case here: rule 2 makes a gapped ring a legal prepared spell and
  rule 3 splits one ring across two objects. Taubin is the same one-pass
  moments with one number subtracted from the diagonal of the solve — `λ = 0`
  *is* Kåsa, so the correction is the entire difference. Points are centred and
  scaled to unit RMS radius first, which is what lets the epsilons be constants.
- `CircleFit { center, radius, rms }` plus `quality()` — `1 − rms/r`, clamped.
  Normalised by radius so "neat" means the same at every size, since canon has
  larger seals stronger and never sloppier. Feeds `Ring::new`.
- Circularity is on the **activation path**, not just the quality score: the
  wiki's Spells page says a ring that is not circular enough gives a fleeting
  effect or fails outright. The compiler will need to gate on `quality()`.
- The overlay draws the fit — fitted circle, centre, ±rms band, per-point
  whiskers at ×5, and the raw centroid in orange. The centroid drifting off
  the centre on an arc is the Kåsa bias, visible. See §0; `debug.rs` still
  reads and never writes.
- Full working: `docs/m4.1-circle-fit.md`.

**Where M4.2 landed:**

- `fit_trimmed` — fits on the survivors, scores on everyone. A pen hook is a
  few samples nowhere near the ring and least squares lets them drag the
  centre, so they are dropped *for the fit*; `rms` still counts them, because
  canon grades a seal on its mess and a score that discards the mess is a lie.
  Bails out to the untrimmed fit when more than 30% would go — that is not a
  circle with strays on it, that is not a circle.
- `Coverage` — largest angular gap around the fitted circle, its heading, and
  `gap_length` in pixels. Closure is judged in pixels because rule 3 joins two
  halves of a seal by physical contact, and a hole is a hole at any radius.
  `Coverage::spans_full_turn(tolerance)` takes the tolerance from the caller:
  core has no idea how wide a pen is. It is **not** the closure test — see
  M4.2b below for why that was wrong.
- Closure is **state, not an event**. Nothing listens for a ring being closed;
  every frame asks whether the gap is under threshold right now. Rule 3's
  toggling then costs nothing — separate the halves and it opens again.
- Known blind spot, recorded as a passing test rather than hidden: sorting by
  angle means a figure-eight and a double loop both report full coverage. Only
  M4.3 can reject them.
- Overlay captions **each** ring where it sits — member strokes, radius,
  quality, and ARMED/CLOSED — because a pad holds several rings at once and one
  list in the corner cannot say which is which.

**Where M4.2b landed** — two bugs the overlay made obvious, both from asking a
stroke at a time:

- `assembly.rs`, `find_rings` — **a ring is not a stroke.** An arc plus the
  short line that closes it is one ring; per-stroke fitting made the line a
  ring of its own with a nonsense radius (a 6%-of-a-turn arc fits any circle
  you like) and left the ring it closed open forever. Grouping is now a query
  over the whole pad, per §3.3: strokes that curve far enough to name a circle
  seed a ring, then every stroke whose ink lies on that circle joins it. Order
  is not an input; a stroke belonging to no ring is absent from the result
  rather than wrong.
- **Closure is decided by endpoints, not by angle.** Angular coverage cannot
  tell a ring whose ends meet from two arcs that overlap in angle without
  touching — from the centre both cover every direction, so the overlay called
  an obviously broken ring closed. Canon is physical about it: the glowstone
  halves complete a spell when they *touch*. A ring is closed when every loose
  end has another end within `join`, and `RingCandidate::open_ends` says
  exactly where a dot of ink would finish it.
- Coverage is still the second signal — a ring can be joined up and still have
  a bite out of it — but it answers `spans_full_turn`, not `is_closed`.
- Metadata the overlay needed and the compiler will want: `CircleFit.max_miss`
  (one dent versus an even wobble — same `rms`, different drawing) and
  `RingCandidate.ink_length` / `.points`. Ink length over circumference says
  how much of the ring was drawn, and over `1.0`, how much was drawn twice — a
  rough stand-in for the turning number until M4.3.
- The overlay's inspector panel names the one number canon does not give us:
  `MIN_QUALITY_TO_FIRE`. Invented, marked as such, and parked in `debug.rs`
  rather than core until the compiler needs a real answer.

**Where M4.3 landed** — the third signal, and the one that rejects:

- `circle::winding` returns `turns` and `sweep`. `turns` is how far round the
  ink went, summed **per stroke** so the leap from one stroke to the next is
  never mistaken for pen travel, and taken as a magnitude per stroke so a split
  seal inked in opposite directions still totals one turn. `sweep` counts
  travel in both directions, so `backtrack = sweep − turns` is zero for a pen
  that only went one way.
- `RingCandidate::is_simple` compares `turns` against `coverage.spanned`
  rather than against `1.0`. For **any** simple arc the two agree, which is
  what keeps rule 2's prepared spell legal — a ring with a deliberate hole is
  not a full turn and must not be rejected for it. A double loop covers one
  turn and winds two; a figure-eight covers one turn and nets nothing.
- Both blind spots recorded in M4.1 and M4.2 are now closed, and the old tests
  stay as-is: they document what each signal *cannot* see, which is why there
  are three of them.
- `RingCandidate::contents` sorts the rest of the pad by canon rule 1 —
  `inside`, `touching` (the *connecting to* clause, and how rule 5 links two
  glyphs), `outside` (does not count). Identity only; what the enclosed ink
  *means* is the recognizer's job.
**Where M4.4 landed** — the ring leaves the overlay:

- **The threshold question had one right answer: units.** Anything measured in
  pixels is a fact about this screen and this pen, so it comes from the shell —
  `CLOSURE_TOLERANCE`, `ON_RING_TOLERANCE`, and the rest of `RingSearch`.
  Anything dimensionless is a statement about magic and belongs to core, so
  `simple_tolerance` and `min_quality` moved into `RingRules`. A ratio means
  the same thing on every screen; a pixel does not.
- `RingCandidate::activation` returns `Malformed | Armed | Fleeting | Active`,
  and the order it asks in is the point. Being a ring at all comes first,
  because "closed" and "neat" are meaningless questions about a figure-eight.
  Then structure before craft: an open ring is *armed* however roughly it was
  inked, because rule 2 makes an unfinished seal a prepared spell rather than a
  bad one. `Fleeting` is the wiki's own word for a ring too rough to hold.
- `RingCandidate::to_ring` compiles the measurement into the `Ring` the engine
  uses. The candidate keeps the evidence — strokes, turning, loose ends — and
  `Ring` keeps only what a spell needs, so a later warning can point back at
  the measurement behind it. A malformed candidate still converts: §4.7 has
  core reporting rather than deciding.
- The overlay no longer decides anything. It had been computing the verdict
  itself, which put a decision about what magic means in a shell (§4.2); now it
  calls `activation` and only chooses wording and colour.

**Where M4.5 landed:**

- `stroke.rs` — `path_length` and `resample`. Capture drops a point every
  `MIN_POINT_SPACING`, which sounds even and is not: a hand slows at curves, so
  points bunch exactly where the drawing is most interesting. That is a bias,
  not untidiness — least squares weights by point *count*, and `$P` compares
  clouds by nearest neighbour, which is meaningless if one is dense where the
  other is sparse.
- Resampling never invents shape. Every point it emits lies on a segment the
  pen actually drew, and both ends stay exactly where they were drawn, because
  endpoints decide ring closure (rule 2) and nudging them is not its business.
- `MATCH_POINTS = 32`, the `$P` paper's own figure. Enough to tell gestures
  apart while keeping the greedy match — `O(n²)` per template — inside the 1ms
  budget across a full catalogue.
- `assembly` dropped its private copy of the length calculation for
  `stroke::path_length`.
- **The fitter does not resample yet, on purpose.** It would change `rms`, and
  so `quality`, and so canon rule 8's grading — that is a decision worth taking
  deliberately with the overlay showing both, not smuggled in here.

**Where M4.6 landed:**

- `recognizer.rs` — `Cloud` and `normalize`. Of the three transforms `$P` can
  divide out, two are exactly what §2.2 asks for and the third would be a bug:
  - **position** removed, **scale** removed — "a sigil's size and location
    within a seal do not change its behaviour";
  - **rotation kept.** Most `$P` implementations turn a cloud to a canonical
    angle. Doing that here would delete canon rule 6 outright — a reversed sign
    inverts its effect, so orientation is meaning, not noise. §5's ±15°
    tolerance comes free instead, because a slightly turned cloud is simply a
    nearby cloud.
- Scaling is **uniform**, by the longer side of the bounding box. Fitting each
  axis to a square separately, as `$1` does for unistrokes, would make a circle
  and an ellipse the same drawing.
- `Cloud` keeps the `scale` and `origin` it divided out. The recognizer must
  not see them and the compiler must — the wiki ties intensity to sigil size
  relative to the ring, and this is the step that would otherwise lose it.
- `stroke::resample` and `path_length` became gesture-aware: segments spanning
  two stroke ids are skipped, so a three-mark sigil resamples as one gesture
  without inventing ink across the gaps between marks.

**Where M4.7 landed** — the greedy match, and the only real speed budget:

- `cloud_distance`, `rank`, `classify`, `Template`, `Match`. The distance is
  the weighted **mean** rather than the paper's weighted sum: the sum grows
  with `n`, so it cannot be read without knowing the sample count, while the
  mean is a fraction of the gesture's own size. Dividing by a constant leaves
  every ranking untouched.
- `classify` is deliberately unthresholded. How close is close enough is a
  question about magic — a sloppy sigil that is still obviously fire should
  probably work — and it belongs to whoever compiles a spell.
- **Scaling moved off the bounding box.** A box is not rotation-invariant:
  turning a square twelve degrees grows its box by a fifth, so dividing by the
  box shrinks the drawing inside it and a small turn changes size as well as
  angle. Most `$P` users never notice because they normalise rotation away
  first; we deliberately do not, and it cost a rotated square its match. RMS
  distance from the centroid has no preference for the axes, so §5's ±15°
  tolerance survives. Same quantity already conditions the circle fitter.
- **The budget was real.** 48 templates at `n = 32` — the catalogue's true size
  — measured 0.87ms against §7's 1ms. Two exact changes brought it to 0.41ms:
  searching on squared distances so each step costs one square root instead of
  `n`, and abandoning a walk once it cannot beat the best already found. The
  bound tightens across directions and starts, so `rank` benefits too. Neither
  changes an answer.
- The measurement was taken with a throwaway test and then deleted: §4.1 says
  core has no clock, and a timing assertion in a debug build measures nothing
  anyway. A real bench belongs in `benches/` when §7 gets one.

**Where M4.8 landed — and what is still missing:**

- `templates.rs` loads recorded gestures from `.ron` and checks every id
  against the catalogue, so a typo cannot quietly become a rune nobody can
  draw. No filesystem: callers pass the file's contents, as `Catalog` does.
- **`the-magic-assets/templates.ron` ships empty, on purpose.** The shapes
  belong to the manga and have to be traced, not invented — §2 is explicit that
  an honest hole beats a plausible guess. A test asserts it is still empty, so
  that emptiness stays a decision rather than becoming an accident.
- Golden tests (§5) use `tests/data/`: six plain shapes as templates, and the
  same six drawn as a hand would — moved, resized, turned a few degrees, noisy,
  several backwards or with the strokes reordered. All six recognise as
  themselves. The fixtures are geometry and are labelled as such; they test the
  machinery, not the vocabulary.
- **A finding for whoever traces the runes.** A square and a ring sit 0.110 and
  0.127 apart at 32 points — a 16% margin. Both are closed convex loops of even
  radius, and 32 samples is not many to tell "has corners" from "does not".
  Doubling `MATCH_POINTS` would separate them and would also cost `2^2.5`, or
  ~2.3ms against §7's 1ms. So the catalogue is better off not holding two runes
  that differ only in roundness.
- **The gap before M5 is a recorder, not code.** Nothing can be matched until
  the shapes exist, and they have to come from tracing panels or from drawing
  them in the app and saving the pad in this format. That is a shell job — core
  cannot write files — and it is not written yet.

- `RingContents::extent` is the one number that must be captured here or lost:
  the wiki ties a spell's intensity to "the size of a sigil in relation to the
  ring", and §3.1 has the recognizer scoring sigils on shape alone with scale
  deliberately thrown away. Nothing downstream could recover it.

**Where M1R landed** — the canon rework, after the telepedia research:

- `catalog.rs` — the `.ron` loader. Every sigil, sign, and spell is data now;
  nothing about behaviour is a `match` arm. Schema in code (`Family`, `Tier`,
  `Confidence`, `Class`, `Slot`, `Scope`, `RegionPattern`), content in
  `the-magic-assets/`.
- `SigilId` / `SignId` — newtypes over `String`. The old `Sigil` and `SignKind`
  enums are gone; an enum of sigils breaks §4.4 the day someone adds one.
- `arrangement.rs` — `Symmetry::classify` (radial by spacing *and* kind period,
  bilateral by mirror axis) and `RegionArrangement::classify`, which returns the
  same `RegionPattern` the spell fixtures record so the two can be compared.
- `Sign.placement` — the angle a sign sits at. Inward and outward are questions
  about direction relative to position, so they are unanswerable without it.
- `Ring::contains(point, tolerance)` — rule 1's *connecting to* clause.
- Core gained `ron` and `serde`. §5's "zero dependencies" is spent; §4.4 and
  data-driven behaviour both required a parser.

**Where M2 and M3 landed:**

- `InkPad` resource — flat `Vec<Point>` plus `stroke_id`, and `undone` for
  history. Flat because `$P` wants stroke membership on the point.
- `capture_stroke` — press/drag/release, gated by `MIN_POINT_SPACING` so a
  still hand doesn't bank sixty duplicate points a second, and by
  `PaperShape::accepts` so ink stays on the sheet.
- `draw_ink` — gizmo polyline, breaking between strokes on `stroke_id`. Width
  is `INK_WIDTH` on the gizmo config, not per call, with round joints so thick
  turns have no notch.
- `shortcuts.rs` — `undo` / `redo` / `clear` / `clear_all` as plain fns on
  `&mut InkPad`, plus `TapCounter`. `main.rs` decides which keys mean what;
  that file decides what they do. ⌘⌫ clears recoverably — the pad goes onto
  `undone` as one entry — and ⇧⌘⌫ wipes everything including the history.
- `PaperShape` — `Full` by default, `Disc` behind **three bare `F` presses
  inside two seconds** (`TapCounter`), because the swap wipes the pad. `extent`
  and `accepts` are the single source of truth for where the sheet's edge is,
  used by both the mesh fit and the pen.
- Palette — `PAPER` parchment on a `DESK` surface, `INK` iron-gall black.
  `place_credit` and `fit_paper` track the window every frame.
- `debug.rs` — the overlay, see §0. Six readouts, layout guides, stroke
  endpoints, F1 to hide.
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
