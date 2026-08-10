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
plausible guess, and §2.6 is where our own ideas are allowed to live.

### 2.1 The parts of a seal

| Part | Position | Role |
| --- | --- | --- |
| **Sigil** | Usually the centre | _What_ — the type of spell |
| **Signs** (keystones) | Around the sigil | _How_ — form, direction, balance, spin |
| **Ring** | Encloses everything | _Activation_ — closes the circuit |
| **Glaives** | Claw-shaped protrusions | _How deeply_ a spell embeds in a body |

**Glaives are neither signs nor sigils** — the Magic page says so outright, so
they are a fourth kind of mark, not a sign with an odd name. They determine
"how firmly a spell will imbed itself into one's body", and whether that means
depth or tenacity is unclear. Nearly forgotten since the Day of the Pact; seen
on memory erasure and slime rendering.

**The sigil is optional.** "While sigils are not required to create a
functional spell, the vast majority contain at least one." Repetition, Billow
and Vision can drive a spell alone, and a glyph with no sigil is a legal spell
the compiler must accept.

**The ring is not optional, and on its own it is already a spell:**

> "A ring is the bare minimum required to produce a spell. If a ring is the
> only thing drawn, the spell generated will simply be a rapid discharge of
> energy, i.e. an explosion."

An empty ring is a bomb, not a no-op. Nothing downstream may treat it as
"nothing to do".

### 2.2 Sigils — families and variants

Sigils are **families**, each holding variants whose behaviour differs from one
another. Behaviour belongs to the variant, never to the family.

**Fire** — flame, heat, light. The page says three variants and lists two.

| Variant | Effect |
| --- | --- |
| **Fire** 炎の紋 | Creates and manipulates flame or heat. Primary tetrad |
| **Light** 光の紋 | Manifests magic as light. A fire variant |

**Water** — one variant.

| Variant | Effect |
| --- | --- |
| **Water** 水の紋 | Manipulates, collects, and creates water. Long-duration spells _collect_ rather than create, implying creation costs more. Primary tetrad |

**Earth** — one variant.

| Variant | Effect |
| --- | --- |
| **Earth** 地の紋 | Manipulates wood, stone, sand, soil. **Cannot create them.** Also called the sigil of might 力の紋. Primary tetrad |

**Air** — four variants.

| Variant | Effect |
| --- | --- |
| **Wind** 風の紋 | Moves and manipulates air. **Cannot create it.** Also called the sigil of levitation 浮遊の紋. Primary tetrad |
| **Aeriforms** 気体の紋 | Creates and manipulates air. **Cannot move it** |
| **Wind Underfoot** 足場のある風の紋 | Supports solid objects suspended in air. Mechanism unclear |
| **Whorling Wind** つむじ風の紋 | Manipulates air through rotation. Three-sided, which may hint at a fire connection — heating the air, as a hot-air balloon does |

**Time** — one variant.

| Variant | Effect |
| --- | --- |
| **Repetition** くり返し | Continuously resets affected objects to the state they held when the spell took hold, temperature included. Spring-like: makes soft things elastic, stops rot, repairs damage |

> Repetition "has been referred to a sign, a seal, and a sigil". Its
> classification is genuinely unsettled in canon, so ours may be too.

**Decorative** — sixteen, and **they are sigils, not signs**:

> "Decorative sigils were previously referred to as decorative signs, up until
> this was **retconned in Chapter 78**."

They are not inert. Three established effects:

1. **Sculpting** — "Spells can be sculpted to take on the rough shape of its
   corresponding design."
2. **Targeting** — "or target other spells shaped by the same sigil."
3. **Restriction** — "They also allow spells to limit their effects to the
   physical counterparts of their respective designs."

The cost is space: "Decorative sigils take up a large amount of space inside a
seal and lack practical utility, leading to most of them being lost to time or
used in a witch's personal doodles." Not absolute — Horse spells "are capable
of pulling heavy loads, suggesting that this sigil has the potential for
practical usage."

`Bird A · Bird B · Dragon · Flower · Horse · Owlcat · Owlcat Head · Scalewolf ·
Torchstag · Liongoat · Valance Leech · Frillram · Sword · Gryphon · Pegasus`

Two structural facts worth more than the list:

- **They decompose.** Owlcat Head is the Owlcat sigil minus the body, which
  "implies that decorative signs can be split into smaller portions which will
  form the individual body parts corresponding to that section".
- **They can take modifiers of their own.** Flower's species "is determined by
  five, small, identical symbols that surround the sigil, acting as modifiers"
  — a level of nesting below the sigil that nothing else in the grammar has.

Gryphon and Pegasus are named only; nobody has seen what they look like.

**Misc**

| Sigil | Effect |
| --- | --- |
| **Guidance** 誘導の紋 | Attracts objects matching parameters set by the other signs and sigils in the spell |
| **Calling** 呼び声の紋 | Repeatedly echoes a recorded phrase. Called a sigil only in Japanese |
| **Obliviation** 忘却 | Function unknown. Only ever seen in memory erasure, usually with glaives |
| **Doorways** 扉の紋 | Likely names the location a spell connects to. Possibly many variants, one per place |

**Unofficial** — fan-named, not established.

| Sigil | Effect |
| --- | --- |
| **Crystalize** | Crystalises air or water into crystal, or into ice. What decides which is unknown, and may be user intent |
| **Smoke** 煙 | Creates and generates smoke. Whether it can also manipulate smoke is unseen |
| **Flickering Light** | Unknown. Coco's failed attempt gave a small firework, so the real effect is probably that but stable |
| **Lightning** | Unknown. Likely creates and manipulates electricity, from its design and its use in a bolt-throwing spell |

> **Critical rule.** "Regardless of a sigil's size or location within a seal,
> its behavior will be the same (with specific exceptions)." So the recognizer
> scores sigils on **shape only**, never on position or scale.
>
> **But size is not meaningless.** "The size of a sigil in relation to the ring
> determines the intensity and strength of the spell's effect, with larger
> sigils creating more powerful effects." Size does not change *what* a sigil
> does; it changes *how strongly*. Both are true and they belong to different
> layers — see §3.1.

### 2.3 Signs (keystones) — the instruction set

Signs control the **form** the sigil's effect takes. Same sigil, different
sign, completely different spell: water + dispersion pours out on all sides,
water + column shoots out like a hose pinched by a finger.

**Forty signs have been identified** — 26 officially named, 14 fan-named —
"with more yet to be deciphered".

#### The four classes

The wiki's own taxonomy, and explicitly *not* the manga's: "These groups are
never mentioned, named, or explained within the source material." Kept anyway,
because the class decides what size and rotation *mean* for a given sign.

| Class | Symmetry | What size does | What rotation does | Invertible |
| --- | --- | --- | --- | --- |
| **Directional** | bilateral, not radial | changes **direction and power** | aims the effect | yes |
| **Semi-directional** | usually bilateral | changes **strength only** | nothing | yes |
| **Non-directional** | radial, not bilateral | strength only | nothing | **no** — no front to point |
| **Asymmetric** | none | unknown | unknown | unknown |

That last column is a validation rule, not flavour: a non-directional sign
*cannot* be reversed, because there is no way to make it point inward.

#### Officially named

| Sign | Class | Effect |
| --- | --- | --- |
| **Columns** 柱の矢 | directional | An area of effect shaped like a column. "The amount of signs will affect the range or quantity of magic generated… Increasing the length of the sign will apply additional pressure or power in a given direction." Inverted, manifests horizontally like dispersion. Also called Signs of Power |
| **Dispersion** 拡散の矢 | unclear | A column that leaks its magic outward instead of beaming it |
| **Levitation** 浮遊の矢 | directional | Floats the target, often shaping it into a sphere when balanced. Length sets how far it travels, what weight it carries, or its speed |
| **Pulling** 引き寄せの矢 | directional | Pulls matter toward the seal when the arrow points inward. **"When pointed inwards at an angle, the spell will have both a pulling and twisting effect, and it's likely that if rotated a full 90 degrees, the spell will just twist without pulling"** |
| **Crushing** 破砕の矢 | semi | Disintegrates objects; "the bigger the sign, the smaller the pieces". Inverted, reforms them — but they revert when the spell ends |
| **Dancing Puppets** 踊る人形の矢 | unclear | Lets a user steer the object it is drawn on, apparently by mind. Movement type follows the sigil |
| **Stability / Level Planes** | non-directional | Balances the target in a plane in air, like a float on water. **Can take the place of a sigil** |
| **Regions** 領域の矢 | directional | Where magic manifests — four cases, below |
| **Convergence** 収束の矢 | semi | Focuses magic to a point; packs loose particles rigid |
| **Stretch** (fan: Weave) | non-directional | Turns solids into long flexible ribbons on contact. Surrounds the central sigil |
| **Coil** | non-directional | Manifests matter in a spring or coil. **Solids only** — no effect on liquids or gases |
| **Cooling** 冷やす矢 | non-directional | Cools things down |
| **Empowerment** 強化の矢 | — | Makes objects stronger, harder, more durable |
| **Focus / Sights Set** 照準の矢 | directional or semi | Aims the spell at a point or target. One of only two signs steered by mind |
| **Entwining** 巻きつきの矢 | semi | Makes the object it is drawn on wrap around other objects |
| **Spiraling Winds** 風の矢 | **asymmetric** | Once served as a wind sigil outright. Function beyond "related to wind" unclear |
| **Aeriforms Defined** 気体の示す矢 | semi | Name and design conflict. Known to modify the wind underfoot sigil |
| **Gathering** 集める矢 | directional or semi | Like collection, but may actively draw material in rather than take what is near |
| **Glaives** | semi | How deeply magic embeds **into flesh**. See the ring exception in §2.5 |
| **Solidification** 凝固の矢 | — | Makes magic drawn within or connecting to it more solid |
| **Binding** 留める矢 | — | Halts the target's movement and binds it into a single unit |
| **Envelopment** 衣まといの矢 | — | Makes an effect envelop what it targets |
| **Concealment** 覆いの矢 | — | Hides a target object |
| **Reflection** 反射の矢 | — | Targets a reflected image found on the object the seal is drawn on |
| **Windows** | — | Connects two spaces not physically connected — a portal. Drawn as **an additional ring** lining the spell, not as a mark beside the sigil. Many variants |

> **Repetition is no longer a sign.** "As of the bonus from Volume 12, the
> status of repetition has been retconned from that of a sign to a sigil." It
> lives in §2.2.

#### Unofficially named

Fan-reconstructed by comparing spells. Plausible, not established.

| Sign | Class | Effect |
| --- | --- | --- |
| **Collection** | directional or semi | Collects material from above and around the seal. Open side faces inward |
| **Billow** | non-directional | Turns available material into a cloud. **Can take the place of a sigil** |
| **Diamond** | non-directional | Affects only *nearby* objects, not the one the spell is drawn on |
| **Selection** | non-directional | Affects only the object the spell *is* drawn on — the mirror of diamond |
| **Enlarge** | semi | Grows the spell (corners out) or shrinks it (corners in) |
| **Crosshair** | directional | Targets whatever the sign's shorter ends point at |
| **Bolt** | non-directional | Manifests magic as bolt-like projectiles that shoot up |
| **Rain** | semi | Produces the sigil's magic as rainfall over the area. Surrounds the central sigil |
| **Orb** | non-directional | A spherical space above the seal where material collects. Fills bottom-up, still under gravity, and only from material added directly |
| **Purify** | **asymmetric** | Separates impurities out of the sigil's magic |
| **Link** | semi | Links magic between the object it is drawn on and everything broken off, removed from, or made of the same original object |
| **Stillness** | — | Holds a magical effect static in one place |
| **Projection** | — | Projects an effect outward. Likely only images the spell specifies |
| **Launch** | — | Generates the target in the direction it points, in a powerful short-lived burst |

**Region's four cases.** Computed from where _all_ region signs point
collectively, not from any one sign:

| Arrangement | Where the magic manifests |
| --- | --- |
| All pointing one side | Shoots that direction |
| All pointing inward | Only inside the ring |
| All pointing outward | Only outside the ring — no effect inside |
| Opposed (some in, some out) | Only _on_ the ring itself (floating drops) |

**Three signs can stand in for a sigil**: Stability/Level Planes, Billow, and
— before its retcon — Repetition. A glyph whose centre holds one of these and
no sigil is a legal spell.

> There is no "decorative" tier of signs. What used to be filed there — bird,
> animal shapes — are **decorative sigils** (§2.2), retconned in Chapter 78.

### 2.4 Balance, size and tilt

The mechanic that makes signs a physical system rather than a list of flags.
All of it is on the Magic page, and none of it was in earlier drafts of this
file.

**A sign's size is its power.**

> "The seal on the left has column signs which are all the same size, and as
> such, the same power. This results in a balanced spell that shoots straight
> up. Conversely, the seal on the right has one column sign which is far longer
> than the others. This longer sign has more power, causing uneven pressure
> which makes the spell shoot off to the side."

So a seal is a set of vectors: each sign contributes a push whose magnitude is
its size and whose direction comes from its placement and rotation. Sum them.
A zero sum shoots straight; a non-zero sum shoots off toward the heavier side.

> "Oftentimes, adding more signs to a spell can help to average them out,
> resulting in a more balanced result."

More signs of the same size average toward zero, which is why real seals are
crowded.

**Best practice is bilateral symmetry.**

> "When making seals, it is generally best practice to maintain at least
> bilateral symmetry to maintain spell stability."

**Tilting signs makes the spell spin, and costs reach.**

> "By tilting the signs within a seal, it is possible to produce a spell that
> rotates. The more tilted the signs, the more spin but less reach the spell
> will have."

A real tradeoff with a knob: tilt trades reach for spin, continuously. This is
the single richest mechanic in the source and it falls straight out of a field
we already have — a sign's `orientation` relative to its `placement`.

**What size means depends on the class** (§2.3), and this is a real branch, not
a nuance:

- **Directional** — size changes power *and* direction. "Increasing the length
  of the sign will apply additional pressure or power in a given direction."
  This is the case that steers a spell sideways.
- **Semi-directional** — "Changing their size will only alter the strength of
  their effect, not direction." Size is a scalar here, and contributes nothing
  to `Balance`.
- **Non-directional** — strength only, and they cannot be reversed at all.

So `balance` must weight by size only for signs whose class puts them in the
first group. A seal of six equal crush signs and one enormous one is *strong*,
not *lopsided*.

**Tilt is confirmed by Pulling, not just by the general rule.**

> "When pointed inwards at an angle, the spell will have both a pulling and
> twisting effect, and it's likely that if rotated a full 90 degrees, the spell
> will just twist without pulling."

A quarter turn converts pull entirely into twist. That is exactly a
`cos`/`sin` split of one push, which is why §2.6 lists the exchange rate as
*derived* rather than invented — canon gives both endpoints and the direction
of travel between them.

**Count matters too, separately from size.** "The amount of signs will affect
the range or quantity of magic generated for a spell." So a seal has three
independent knobs: how many signs, how big each is, and how far each is tilted.

### 2.5 Ring and structural rules

Each becomes a real rule in our engine.

1. **Everything must be inside the ring, or connecting to it.** "All sigils and
   signs in a spell must be drawn within or somehow connecting to a spell's
   outer ring. If they are not, they won't contribute towards the effect of the
   spell." Note the _connecting to_ clause — containment cannot be a pure
   point-in-circle test.

   **Glaives are the exception**: "Unlike any other known sign, glaives can be
   drawn outside of the ring as long as they are still connected to it." So
   *connecting to* is not a courtesy for sloppy drawing — for one mark it is
   the normal way to draw it.

2. **A spell activates only when its ring is complete.** Leaving a gap prepares
   a spell to be fired later by closing it.

3. **Spell toggling.** Draw one spell in two parts across two objects. Touching
   them completes the ring and turns it on; separating turns it off (the
   glowstone path).

4. **Nesting.** Wrap a spell in a second ring and fill the gap between them
   with another spell to combine both effects — on the same object or across
   separate ones. **The activation order is settled**: "In nested glyphs, the
   inner ring will only activate if the outer ring is completed, even if there
   is no gap in the inner ring." The outermost ring gates everything inside it.
   The *mechanism* remains unknown — "seals can somehow tell whether or not
   they're surrounded by an incomplete ring" — but the behaviour is not in
   doubt, and we no longer model it as ambiguous.

5. **Linked spells.** Two glyphs joined by a line link their effects. Identical
   or similar linked glyphs amplify each other — "when several small, identical
   seals are linked together, their combined strength will often be more than
   would be possible for a single large spell that took up the same amount of
   space".

6. **Reversed signs invert their effect.** A normal spell and its reversed twin
   cancel out completely. Some forbidden spells are exempt, for unknown reasons.

7. **Symmetry.** Signs are usually arranged in radial or bilateral symmetry.
   Asymmetric spells are valid but unstable — see §2.4 for what instability
   actually does.

8. **Quality is geometric.** "The size and precision of a seal have a strong
   effect on the quality of a spell: larger seals are more powerful than
   smaller ones, and neatly drawn seals are more stable and long-lasting than
   messy ones."

9. **A bare ring is an explosion.** §2.1. The empty case is not the trivial
   case.

**Nothing above constrains drawing _order_.** Canon never says the ring comes
first, and rules 2 and 3 both describe seals whose contents exist before the
ring closes — an unclosed ring is a fully prepared spell waiting on its last
stroke, and half a split seal is drawn with no complete ring at all. A player
who draws the sigil, then the signs, then the ring last is doing the canonical
thing, not a weird thing.

So the engine never enforces an order, and never rejects a stroke for arriving
too early. It also never _forbids_ ring-first — both orders, and every order in
between, produce the same glyph. See §3.3 for what that requires of the types.

### 2.6 What we invent (clearly marked, not canon)

Canon never explains _why_ magic works — it's a hard system with defined
components but no underlying mechanism. So the following is **our extension**,
and we should be honest about that in the README:

- **Reactive elements.** Canon doesn't have "wind + water = tornado." We add an
  emergent reaction layer where element instances interact via temperature,
  velocity, and density fields.
- **Continuous physics.** Canon spells are discrete effects. Ours run in a
  simulation with gravity, heat transfer, and phase change.
- **Numbers for the qualitative rules.** Canon says a seal that is not circular
  enough fizzles, and that a longer sign has more power. It gives no
  thresholds. Every constant we pick — `min_quality`, how much lean counts as
  unbalanced — is ours, and must be marked as ours where it lives.

Three things that are _not_ on this list, though earlier drafts said they were:

- **Rotation is canon on both sides.** Whorling Wind rotates as an element, and
  §2.4's sign tilt rotates as a modifier. The claim that "rotation lives on the
  sigil, so we must not have a rotate modifier" was wrong: tilt *is* the
  modifier, and it is already expressible as a sign's orientation relative to
  its placement.
- **The spin/reach exchange rate is derived, not invented.** Pulling at a right
  angle "will just twist without pulling", and at an intermediate angle does
  both. Two endpoints and a monotone path between them is a `cos`/`sin` split
  of one vector. We chose the smooth curve; canon chose the endpoints.
- **Sign power is canon.** Size means power, and unbalanced sizes steer the
  spell. We are reading the Magic page, not inventing a physics.
- **The capability model in §3 is derived, not invented.** `can_create` /
  `can_move` fall straight out of the wiki's own wording: wind moves air but
  cannot create it, aeriforms creates air but cannot move it, earth manipulates
  but never creates.

Rule of thumb: **canon defines the grammar, we define the semantics.** Never
break a canon rule for convenience — they're better constraints than anything
we'd invent.

---

## 3. How canon maps to our types

Live in `magic-core`. Do not let Bevy types leak into these.

```
SigilId     ← names a sigil in sigils.ron
SignId      ← names a sign in signs.ron
Sign        ← SignId + placement + orientation + size + reversed
Glaive      ← neither sign nor sigil (§2.1); how deeply a spell embeds
Ring        ← center, radius, closed: bool, quality: f32
Glyph       ← one spell (optional sigil + signs + glaives + ring + nesting + links)
Balance     ← the vector sum of a sign set: where the spell will actually go
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

A sigil's **position** never matters, and its **shape** is the only thing the
recognizer may score on (§2.2). Its **size** is different: it does not change
what the sigil does, but the size of a sigil *relative to its ring* sets the
spell's intensity. So scale is divided out for matching and kept alongside —
`Cloud::scale` and `RingContents::extent` exist for exactly this, and nothing
downstream could recover either if they were dropped.

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
  meaningless without them — plus a **`placement`**, the angle it sits at
  around the ring. Inward and outward are questions about direction _relative
  to position_, so the same sign at the top and at the bottom of a ring means
  different things. Rule 1's containment test needs the position too.
- **`Sign` also needs `size`.** §2.4: a sign's size *is* its power, and one
  sign longer than its neighbours steers the whole spell sideways. Without it
  `arrangement.rs` can measure symmetry of position and never symmetry of
  power, which is the half that decides where the magic goes. Recorded in the
  same units as the ring's radius, so the two are comparable.
- **Balance is a function over the sign set, not a per-sign property.** Each
  sign is a vector: magnitude from `size`, direction from `placement` and
  `orientation`. Their sum is where the spell shoots. Zero is balanced; adding
  more equal signs drives it toward zero, exactly as the wiki describes.
- **Tilt is `orientation` measured against `placement`**, and it buys spin at
  the cost of reach (§2.4). The type already carries both angles; what is
  missing is only the reading of them. **No rotate sign should ever be added** —
  not because rotation is not a modifier, but because tilt already is one.
- **Glaives are their own kind.** Not a `SignId`, not a `SigilId`. They sit in
  their own list on `Glyph` and answer one question: how firmly the spell
  embeds in a body.
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
  billow, or vision compiles fine. So does one with **nothing at all** inside
  the ring: rule 9 makes that an explosion, and the compiler must return that
  spell rather than an empty one.
- **Decorative sigils are sigils** (§2.2), so they need no new type — a
  `Family::Decorative` and the ordinary `SigilId` cover them. Their three
  effects (sculpt, target, restrict) are `caps`-like data, and their two
  structural oddities are not yet modelled: that they decompose into body parts,
  and that Flower takes five modifier symbols of its own. Both are honest holes
  until a spell needs them.
- **Glyph assembly is geometric, never chronological.** Grouping strokes into a
  glyph is a query over the finished pad — find the closed loops, take
  everything each one contains or touches (rule 1), classify the rest. Stroke
  order is not an input, which is the same reason the recognizer uses $P.
  Concretely: `Glyph::new` must stop demanding a ring at construction, since
  that signature alone makes ring-first the only expressible order.
- **Strokes belonging to no ring are inert, not invalid.** They stay on the pad
  unclassified. A player halfway through a seal has drawn nothing wrong.
- **Quality** is an `f32` derived from stroke neatness, on everything — rule 8.
- **Nesting is decided, and the type should say so.** Rule 4: the outermost
  ring gates every ring inside it. An inner ring with no gap still waits on the
  outer one. This was modelled as an ambiguity in earlier drafts; it is not one.
- **Compilation returns a spell _plus warnings_.** Unstable is not an error. An
  asymmetric seal compiles and carries a warning saying which way it will
  drift, computed from `Balance`.

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
| The same seal compiles identically drawn ring-first and ring-last      | §2.5 — order is never an input         |
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
│           ├── sigils.ron    34 sigils + capabilities
│           ├── signs.ron     44 signs, three canon tiers
│           └── spells.ron    13 spell fixtures + 7 edge cases
└── apps/
    └── canvas/           Bevy shell
        └── src/
            ├── main.rs      app, InkPad, capture, paper, ink
            ├── shortcuts.rs undo / redo / clear on &mut InkPad
            ├── debug.rs     on-screen overlay — Claude's, see §0
            ├── ui/          tool panel: place, guides, toggles
            │   ├── mod.rs   plugin, ToolState, the TOOLS table
            │   ├── bar.rs   panel geometry, drawing, hit-testing
            │   ├── place.rs pick-then-place: drag, preview, commit
            │   ├── guides.rs helper rings and spokes
            │   └── stamp.rs generating exact ink
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

**Canon rework, after reading the wiki first-hand.** WebFetch gets 402 from
telepedia; the pages open fine through the browser, and §2 is now written from
the real Magic, Sigils Explained and Signs Explained rather than from search
snippets. What changed:

- **Decorative sigils are sigils, and they have effects.** Retconned from
  signs in Chapter 78. Fifteen of them, each able to sculpt a spell into its
  shape, target other spells built from the same sigil, and restrict a spell to
  that shape's real counterparts. `bird` and `animal_signs` are gone from
  `signs.ron`; `Family::Decorative` and fifteen entries are in `sigils.ron`.
- **Repetition was retconned sign → sigil** in the volume 12 bonus, so it is no
  longer in `signs.ron` at all.
- **`Sign` gained `size`, because size is power.** Canon: identical column
  signs give "the same power… a balanced spell that shoots straight up", one
  longer sign "has more power, causing uneven pressure which makes the spell
  shoot off to the side". `arrangement::balance` sums the sign set into where
  the spell will actually go, and `spin` reads the tilt/reach tradeoff.
- **Only directional signs steer.** For semi-directional signs "changing their
  size will only alter the strength of their effect, not direction", and
  non-directional ones cannot be inverted at all. `balance` and `spin` take a
  predicate saying which signs steer.
- **The spin/reach split is derived, not invented.** Pulling at a right angle
  "will just twist without pulling" — canon gives both endpoints, we chose the
  smooth curve between them.
- **Glaives are the exception to rule 1** — the only mark that may be drawn
  outside the ring, so long as it still connects. And per Engendale they are
  "not technically signs" but an older technique.
- **A bare ring is an explosion** (rule 9), and **nesting is no longer
  ambiguous** — the outermost ring gates every ring inside it.
- **Both vocabularies now cover everything the wiki names.** 34 sigils against
  32 named, 44 signs against 40 headings. The extras are older fan names the
  current pages have dropped — `unburning_flames`, `stop`, `bend`, `eye`,
  `vision`, `radial`, `float` — kept because spell fixtures reference some of
  them, and each marked in the data as unlisted rather than passed off as
  canon. `every_sign_the_wiki_names_is_in_the_catalogue` walks the wiki's own
  table of contents, so a sign going missing surfaces there rather than as a
  spell that silently cannot be built.
- **`window` was renamed `selection`.** Canon's Windows is a *portal* sign that
  lines the seal as a second ring; what we had called `window` is Selection,
  which restricts a spell to the object it is drawn on. Leaving the old name
  would have collided head-on. The rename turned up a dangling reference from
  `enlarge`, which the catalogue's cross-check caught at load.

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
