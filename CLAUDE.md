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

8. **Every core capability gets a way to reach it from the panel.** A rule
   that can only be exercised by drawing it by hand is a rule nobody will
   exercise. So a change to `magic-core` is not finished until `apps/canvas/ui`
   can produce the input it needs and show the answer it gives:

   - a new **kind of mark** (sigil, sign, glaive, nested ring, link) gets a
     `Shape` and a `place` button, so a seal using it can be built without
     drawing skill;
   - a new **reading** (balance, spin, region, a compile warning) gets a
     `Toggle` and a line in the overlay, so it can be seen changing as the seal
     changes;
   - a new **threshold** we invented gets a `Command` to nudge it, so the
     number can be argued with on screen rather than recompiled.

   Two edits, both in `apps/canvas/src/ui/` — the table in `mod.rs`, and the
   behaviour in `stamp.rs` or `run`. The panel lays itself out from the table.
   Record the pairing in §9 alongside the milestone: *what landed in core*, and
   *which button reaches it*.

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
├── web.sh                builds the wasm bundle and serves it
├── web/index.html        the page: a desk, a canvas, and nothing else
├── crates/
│   └── magic-core/
│       ├── src/
│       │   ├── lib.rs        Point, re-exports
│       │   ├── recognizer.rs $P point-cloud recognizer
│       │   ├── shapes.rs    what each mark LOOKS like — see §12
│       │   ├── naming.rs    ink -> sigil + signs. The join that was missing
│       │   ├── circle.rs     ring fitting + closure detection
│       │   ├── glyph.rs      Sign, Ring, Glyph — identity, no behaviour
│       │   ├── catalog.rs    .ron loader; the only thing knowing behaviour
│       │   ├── arrangement.rs symmetry + region analysis over a sign set
│       │   ├── assembly.rs   strokes → rings; closure by endpoints
│       │   ├── stroke.rs     path length + even resampling
│       │   ├── templates.rs  recorded gestures, loaded from .ron
│       │   ├── compiler.rs   Glyph → Spell, with validation
│       │   ├── sim/          the world a spell acts on
│       │   │   ├── vec2.rs    a 2-vector, and the operators on it
│       │   │   ├── material.rs what a substance IS — density, phase, drag
│       │   │   ├── reaction.rs what happens when two of them meet
│       │   │   ├── parcel.rs  SubstanceId, Parcel, Anchor
│       │   │   ├── field.rs   Cell, Field — the grid, and where mass moves
│       │   │   ├── step.rs    SimRules — motion, heat, repetition, expiry
│       │   │   ├── cast.rs    CastRules — Spell → matter, under §3.2
│       │   │   └── mod.rs     Sim — a world, its rules, how far it has run
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
            ├── sim.rs       the world in the app: FixedUpdate 60Hz, cast_pad
            ├── hand.rs      the typeface, baked in — see §11
            ├── reading.rs   the pad, read ONCE a frame, shared by everyone
            ├── debug.rs     on-screen overlay — Claude's, see §0
            ├── ui/          tool panel: place, guides, toggles — see §4.8
            │   ├── mod.rs   plugin, ToolState, the TOOLS table
            │   ├── bar.rs   panel geometry, drawing, hit-testing
            │   ├── place.rs pick-then-place: drag, preview, commit
            │   ├── guides.rs helper rings and spokes
            │   ├── tutor.rs  the guide: draw a seal one stroke at a time
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
M5  Compiler ring     ██████████ 8/8   ✅
M6  Elements/physics  ██████████ 8/8   ✅
M7  Reactions         ██████████ 8/8   ✅
M8  Web build         ████░░░░░░ 3/7   DROPPED
M9  Camera & vision   ░░░░░░░░░░ 0/7   DROPPED
M10 AR & polish       ░░░░░░░░░░ 0/7   DROPPED
--- the milestone track ends at M7 ---
P   Polish            ░░░░░░░░░░       ← current, and open-ended
```

**M7 is the last milestone. M8, M9 and M10 are dropped**, on Daffa's call: "in
other word we're done here m7 is the last part, and all we need to do left is
just to polish and fix all of the bugs that still here, and improve everything
to the max, and optimize everything and after that we can build it up and get my
friend try."

So there is no web build, no camera, no AR. What is left is not a milestone with
a task count — it is **polish, bugs, and speed, until a friend can be handed the
app**. That is a real change of shape and the plan should say so rather than
pretend the numbered track continues.

Three consequences worth writing down before they are forgotten:

- **The wasm work stays and stays honest.** M8.1 to M8.3 are done and every
  `cfg(target_arch = "wasm32")` branch in the tree is still correct. None of it
  is removed, because §4.1's "core must compile to wasm untouched" was never
  about shipping a web build — it is the constraint that kept `std::fs`, clocks
  and randomness out of core, and every one of those refusals still pays. What
  goes is the *plan* to finish a browser shell, not the discipline.
- **"A friend tries it" is now the acceptance test**, and it is a harder one
  than a task list. It means: nothing on screen lies, nothing needs explaining
  before it works, and no button does something a person would not expect.
- **`./run.sh` is the delivery path**, not `./web.sh`. macOS gives unbundled
  binaries no activation policy, so a real `.app` is the only build that gets
  keystrokes.

M4 came in at nine tasks, not the eight first planned: M4.2b — assembling
strokes into rings — was scoped as part of M4.2 and turned out to be its own
piece of work.

**Current task:** M8.4 — touch input. M8.1 to M8.3 are done: core and the shell
both compile to wasm, and `./web.sh` builds and serves a page. What is untested
is a finger rather than a mouse, and a pad you cannot draw on with a finger is
not a web build worth having.

**The one thing still blocking the recognizer is not code.** `templates.ron`
ships empty and the shapes have to be traced. `record` writes them now, so the
step is a person tracing panels, not a missing feature.

**Every real seal was unreadable, and rotation was why.**

The end-to-end test found it in one line: a stamped `flamespout` read back as a
**water** sigil. Printing the segmentation showed the rest —

```
flamespout   12 stroke(s) -> ["water:Sigil", "levitation:Sign", "?", "column:Sign", "fire:Sigil"]
fire_shot    10 stroke(s) -> ["?"]
```

— which is not a near miss, it is nonsense. And the cause is a decision made
back in M4.6 and defended ever since: **`$P` keeps rotation, because canon rule 6
makes a reversed mark do the opposite of an upright one.**

For a sigil that is right and stays right. For a **sign** it is unworkable, and
it should have been obvious the moment `water_orb` existed: four keystones
arranged around a ring point in four different directions, so **at most one of
them can ever sit at the template's own angle**. That is not an edge case — it is
what §2.4's entire balance mechanic is made of. Three of every four arrows were
unreadable, the leftovers merged into blobs that happened to score near some
other rune, and the seal compiled to a discharge and was refused.

`recognizer::rank_aligned` scores a gesture against each template at *that
template's* angle, found by the **principal axis** of the point covariance —
order-invariant, scale-invariant, and defined up to 180 degrees. It tries both
ways round, because that ambiguity is exactly what canon rule 6 is about: a sign
and its reversed twin lie on the same axis and do opposite things. Two distance
computations per template rather than one; it does not sweep angles, which would
be twenty times the work for an answer the covariance gives exactly.

`principal_axis` returns `None` for a mark with no direction, and that is a real
answer rather than a failure — §2.3 says outright that a non-directional sign has
"no front to point", so inventing an orientation for a radial mark would be
inventing a rule canon denies.

`naming::read` now runs both rankings and takes each rune's answer from the one
its kind calls for. Two tests pin the pair, and the second matters more than the
first: `keystones_are_read_at_whatever_angle_they_are_drawn`, and
`a_sigil_is_still_matched_upright` — because a fix that let everything rotate
would pass the first and quietly delete canon rule 6.

**Two smaller things the same test caught:**

- `stamp::rune` scaled by the recogniser's RMS, which says nothing about where a
  shape's *tips* land — so an oversized sigil touched the keystones around it,
  and `naming` segments on the gaps between strokes. It scales by the furthest
  point now, so `seal` can promise the sigil fits inside a quarter of the ring
  while the keystones reach in to half of it.
- `naming::read` was running `normalize` and `rank` a **second** time purely to
  recover the distance for the DP's score — doubling the cost of the whole search
  for a number the first call already had in its hand.

**The test that found it could not have been a unit test.** Every piece was
already covered and every piece was working: the stamp drew, the ring search
found, `naming` segmented, the compiler compiled. The seal still failed, because
the geometry one part chose was input another part should never have been given.
`seal_round_trip.rs` presses `preset`, reads the pad back, and asserts the spell
that comes out is the spell the button promised — which is the test a friend runs
without knowing it.

**A ring closed, nothing happened, and nothing said why.**

Reported as "some of the particle won't run even tho the ring is closed", and it
was not the particles. Core emits an event for every outcome — a scratch test
confirmed all six sigils announce something, including earth's `Refused` when
there is no stone in reach. What was missing was in the shell: `watch_the_pad`
reported a cast and said **nothing at all** about a seal it had declined.

That was a hole I made in the same change that added the refusal. A ring that
closes and does nothing, with nothing on screen saying why, is indistinguishable
from a ring that closed and the app missed it — which is exactly how it got
reported.

Two fixes, and the first is the one worth remembering: **the rising edge is
about the ring, not about the cast.** `Armed::fired` now tracks whether the
circuit was closed last frame, so a refused seal has still had its one moment and
does not re-ask sixty times a second. A refusal reprinted every frame is the same
as no refusal at all.

The second is where it appears. The panel gets `ring closed, nothing cast - ...`,
and the **caption over the seal** — the closest text to the drawing — now says
`NOT CAST - ...` where it used to say `DISCHARGE - a blast` about a spell the app
was quietly never going to fire. A caption confidently naming a spell that will
not happen is worse than a blank one.

`announce` also lost an early return: a `Fired` outcome that raised zero mass
emitted no event at all, which made a strange thing a person should be able to
*watch* happen look identical to a cast that never occurred.

**There were two recognisers, and they disagreed.**

Reported as "the recogniser IS NOT SYNC at all, so it got it all wrong", and it
was exactly that. `debug.rs` kept a private `Runes` resource — the shipped file
plus the recorder's — while `reading.rs` built its own list of the same two
*plus* the built-in reconstructions. Two vocabularies, ranked separately, so the
board scored a drawing against eight shapes while the compiler beneath it scored
the same drawing against eleven. Neither was wrong about its own list. A readout
that does not describe the thing it sits next to is worse than no readout, and
this one had been lying since the day the built-ins landed.

`Reading::shapes` is now the only such list in the app, with `sources`
index-aligned beside it so the board can still say where each shape came from.
`Runes`, `reload_runes` and `reload_from_disk` are deleted. §0 made that cheap:
the overlay is allowed to read app state and never to keep its own.

**Canon rule 9 is still in the engine. This app declines to fire it.**

Reported plainly: "BLAST is still can be casted even tho we're not records any
of it, so i want you to remove it". The tension is real — rule 9 is one of the
firmest things the wiki says, and §2 forbids breaking a canon rule for
convenience — so the refusal is a **tool policy in the shell**, not a change to
the compiler. `compile` still returns `Driver::Discharge` for a bare ring, still
reports full intensity, and every discharge test still passes. `sim::refuses` is
what the app consults before casting.

The reason is the workshop rather than the fiction. *Every* ring holding ink the
recogniser cannot read compiles to the same discharge as an empty one — so while
the vocabulary is a handful of runes deep, "blast" is what almost every misread
drawing does. A spell that goes off when the app did not understand you teaches
nothing and buries the thing you were trying to look at. **A seal fires when it
was understood.** The refusal even distinguishes the two cases, because they
want different advice: a bare ring wants a sigil drawn, a ring full of unread
marks wants the sigil it already has *traced*.

`traced` on the panel turns it off and puts rule 9 back, which is the honest way
to hold a canon rule and a working tool at the same time.

**Built-in sigils are excluded while `traced` is on; built-in signs are not.**
The asymmetry is the point rather than a compromise. A sigil decides *what* the
magic is, and §12 is explicit that these shapes are stand-ins — reconstructions
of a community project's reconstructions — which is not a thing to fire a spell
on by accident. A sign decides what *shape* the magic takes, so a misread
keystone gives you the wrong spout rather than the wrong element, and keeping
them is what lets a seal be built at all before anybody has traced forty-four
keystones.

**The preset was the one button that bypassed the recogniser.**

It stamped a generic cross-and-triangle in the middle and then set a naming
override so the seal would compile to something. So the button most likely to be
pressed by somebody learning the app was the button that never went through the
recogniser at all — and it was *also* the reason a preset could cast magic
nobody had traced.

`stamp::rune` un-normalises a recorded template back onto the pad: `Cloud`
divides out position and scale and keeps both, so putting a rune back is the
same arithmetic run backwards, and `stroke_id` survives normalisation untouched
— which matters, because `naming` segments on the gaps between strokes. So a
preset now draws **your** traced fire, and names itself the way a hand-drawn seal
does. Where there is no traced rune it declines and says which one to go and
trace.

That let the whole naming override go: `sigil >`, `sigil <`, `spell >`,
`spell <` and `unname`, plus `ToolState::sigil`, `ToolState::fixture`,
`apply_naming`, `fixture_signs` and `preset_fixture`. It existed because
`templates.ron` shipped empty and the compiler was otherwise unreachable from
the app — a real problem then, and solved by the thing it was standing in for.

`PRESETS` became a struct on the way, having outgrown a positional tuple two
fields ago, and lost `everlasting`: a fine spell built on the repetition sigil,
which is to say a button that could only ever be refused.

**A spell now runs for between fifteen and forty-five seconds**, and the floor is
the half worth arguing about. Rule 8 grades a seal on how neatly it was drawn,
and grading one down to a splash that is over before you have looked at it
teaches nothing — a rough seal should be visibly *worse* than a neat one and
still be a spell. `Fleeting` is the one exception and keeps its short life,
because canon's own word for a ring too rough to hold has to cost something.
`a_seal_that_does_not_fire_channels_for_no_time_at_all` pins the other end: an
open ring is *prepared* (rule 2), and a floor applied before that check would
have handed it fifteen seconds of summoning.

**Where the power is aimed, which `balance` could never say.**

Asked for directly: "if we want to make a water orb we need to focus all of that
water power using 4 signs arrow so why don't we have a parameter to show such
that". There was no such reading, and the gap is sharper than it looks — four
arrows pointing at the middle sum to **zero drift**, so `Balance` correctly
reports a spell that goes nowhere and is structurally incapable of saying that
all four are pushing at one point. That is the entire difference between a water
orb and a fountain.

`arrangement::focus` reads the sign set a second way: each steering sign is a
ray, and the answer is the **least-squares intersection** of those rays — the
point whose total squared perpendicular distance to every ray is smallest. In
two dimensions each ray contributes `n nᵀ` to a `2×2` matrix and the solve is a
determinant; a singular matrix means every normal is parallel, which means every
ray is, and that is the beam case detected rather than divided by. Weighted by
size, because size is power (§2.4) — one arrow drawn longer drags the focal point
toward what it is aimed at, which is the same claim canon makes about a longer
column sign steering the whole spell.

**The four convergence cases turned out to be canon's four region cases**, and
that is a finding rather than a definition. §2.3 states them for region signs
specifically; they fall straight out of asking where *any* set of directional
signs points. Which is why a water orb — four levitation arrows and not one
region sign — used to report `region Absent` while obviously converging.

What it cannot know is stated where it lives: `orientation` from a hand-drawn
mark is an *axis*, and which end is the arrowhead is what `reversed` is for. A
line and its reverse intersect at the same place, so the focal *point* is
reliable; converging and diverging are exactly the pair that swap when a sign is
read backwards.

**Forty-three buttons, down from fifty-four.** Gone: the five naming-override
buttons above, `cross`, `bar` and `triangle` (a mark to put in a ring — `sign`
and the five element stamps cover it), `paper` (still three taps of `F`),
`sweep` (folded into `empty`), and `guide >`. That last one was two selectors for
one intention: the guide teaches whatever `preset >` has chosen, so the button
that stamps a seal and the guide that teaches it can no longer name different
spells. `spiral` stayed despite being a test shape, because it is the only way
to produce a not-a-ring from the panel and §4.8 asks for exactly that.

**The seal was drawn right and the compiler was told nothing.**

Reported from a screenshot: an earth sigil inside a closed ring, reporting
`DISCHARGE - a blast` with `3 mark(s) inside that nothing can name` — while the
recogniser board, on the same frame and the same ink, read `earth 0.081, margin
0.086, a clear call`. **Two answers from one drawing**, which is always the
signature of a metric that does not match its own rule.

The board scores the ring's whole contents as one cloud. `naming` first split
them into marks, and split them by the distance between **stroke centres** — but
an earth sigil is a bar over a chevron, and those two strokes *touch* while their
centres sit most of the mark apart. One sigil became three pieces, none of which
resembled anything.

Measuring the gap between the strokes' **ink** instead is the obvious fix and it
is also wrong, and the traced runes are what proved it. The widest gap inside one
real rune is **35px** — water is three teardrops drawn well apart. The gap
between a sigil and the keystone beside it, in a seal that has to segment
correctly, is **32px**. There is no threshold between 35 and 32. Any single
number either shreds water or swallows the keystone.

So segmentation stopped being a question about geometry alone. **Geometry
proposes and the recognizer disposes**: merge strokes agglomeratively by ink gap
to get a binary tree — every spatially sensible grouping and no others, `2n-1`
nodes rather than `2^n` subsets — score every node by asking `$P` what it is, and
take the best cut by dynamic programming. A blob loses to its parts when the
parts read as something and wins when they do not. Water's teardrops merge
because separately they are nothing; the sigil and the keystone stay apart
because together they are nothing. Same rule, opposite answers, which a threshold
could never do.

Two constants carry it and both are ours. `UNREAD_COST` is the exchange rate
between reading fewer things well and more things badly. `SPLIT_COST` is a
parsimony prior, and it was not in the first version: the built-in light sigil is
a diamond inside a square, and **the square alone matches `light`** nearly as
well as the whole thing, so two marks each called light outscored one and the
seal reported a sigil it could not place. A part of a rune resembling the rune is
a discovery about the shape, not a second rune.

All eight traced samples now segment to exactly one mark and read as themselves.
`a_sigil_and_a_keystone_beside_it_stay_two_marks` pins the other direction,
because a fix that merged everything would pass the first test and be worthless.

**`trace >` could not reach a single sign, and never could.** `record` has always
indexed `traceable` — sigils and then signs — while the button stepped modulo the
*sigil* count. Thirty-four of forty-four keystones were unreachable from the
panel. Found by sweeping the two against each other rather than from a
screenshot, which is the only way a bug like that surfaces: nothing misbehaves,
a list just quietly ends early.

**Props: something for a spell to act *on*.**

`fill_air` fixed half of "it fires and nothing happens" — a discharge needs
something to shove. This is the other half. `sim/prop.rs` is a rectangle of some
substance sitting on the paper that catches, soaks, is blown about and is lit,
and `kindle` scatters a handful.

- **Entirely ours** (§2.6). Canon has no objects and never says what a spell does
  to a plank.
- **A prop is not a parcel, on purpose.** A hundred `wood` parcels would be less
  code and would drift apart within a second, because parcels are a *fluid*. The
  point of a prop is that it holds together until something destroys it.
- **Burning conserves.** A plank of mass six leaves exactly six units of flame
  and smoke behind it, drawn from its own integrity — §3.2 would be a lie
  otherwise the moment objects existed. `Sim::prop_mass` exists because
  `Field::mass` alone would show a world that invents matter.
- **`ignites_at` is data, and its *absence* is load-bearing.** Stone has no entry
  rather than a high number, which is what makes a stone plinth safe to stand a
  fire on. `cloth` was added at 180 degrees as the clearest demonstration of
  wetness, which refuses ignition rather than merely cooling it.
- **Scattering is a hashed pure function of the index** (§4.3), so "random on the
  paper" is reproducible and a scattered board is something a test can be written
  against at all.

*Reaches it from the panel (§4.8):* **`kindle`** scatters, **`kindle >`** chooses
wood, cloth, stone, ice or sand and says what each refuses to do, **`sweep`**
clears them. The world block grew a `props` line — count, mass, how many alight,
soaked and lit.

**Moments needed their own representation.**

A blast is over in one tick. By the frame after, the only trace is that some
parcels are moving outward, which is not something a person can see — so a
renderer that only draws *state* can never show what happened. `sim/event.rs`
says what occurred and `apps/canvas/src/particles.rs` decides what that looks
like, which is §4.2 drawn through a new place.

- **The look table is complete, not merely used.** Every substance in
  `materials.ron` and every `Event` variant has an entry, including ones nothing
  emits yet, because a hole in it fails *silently*: the effect does not appear
  and nobody can tell a missing rule from a missing colour.
- **Gathering falls inward, creating blooms outward.** The capability model
  (§3.2) made visible — you can see whether a spell made its substance or took it
  from the room without reading a number. `Refused` is grey and falls, because
  §3.2 biting has to look like a failure rather than like nothing.
- **The queue is drained, not read.** `Sim::step` deliberately does not clear it:
  a fixed step can run twice between two frames, so a tick's events would be gone
  before anything saw them. A cap stops a shell that never drains from growing.
- **Nothing here is read back.** No mote touches a parcel, a prop, a field or a
  spell, which is why a file that runs on the frame clock and hashes a counter
  does not violate §4.3. That is the test for anything added to it.
- **Reactions got a position.** `ReactionReport::fired` was `(String, f32)` and
  could only ever be printed in a corner; it is a `Fired` with an `at` now, so
  water hitting fire is visible where it happened.

**Light had to be both halves.** The desk dimming was already there and on its
own it is a room going dark for no reason. `draw_glow` is the light itself: a
radial falloff texture generated at startup — six lines of arithmetic, and §11
keeps the repository's only binary the typeface — drawn under the ink for
everything whose material `glow`s, and for every burning prop. The paper dims too
now, which reverses an earlier call: leaving it alone did keep the ink readable
and it also made the *unlit page* the brightest thing on screen during a light
spell, which is backwards. It goes a third of the way and no further.

**The tracing board is its own panel now.**

`trace` on the `view` section, beside `spell` and `world` rather than behind F1,
because tracing runes is a *sitting* and reading six blocks of measurement to
find one number is not a thing to ask of somebody mid-stroke. It shows what
`record` will save — the same non-ring selection the recorder itself uses, or the
percentage would describe a different drawing from the one that gets written —
how many samples of each rune are on file, the three nearest matches, and the
verdict against the rune you *said* you were tracing, which is the thing worth
knowing on the occasions it is not the one that won.

**Closeness is a percentage with its threshold named.** `100%` is exact and `0%`
is `naming::NEAR_ENOUGH`, the distance at which the compiler stops believing it.
It is not a probability and does not pretend to be one — it is where this drawing
sits between perfect and rejected, which is the question a person tracing is
actually asking. The recogniser board's ranking carries the same column.

**Where the recogniser board landed** — `$P` gets the overlay the circle fitter
already had:

- **The board ranks every rune, not just the winner.** A single "best match" is
  the one number that cannot be argued with, and the interesting question is
  always what came *second* and by how much — M4.8's own finding was that a
  square and a ring sit 0.110 and 0.127 apart, which is only visible if both are
  on screen. So the board prints distance, a bar and a source for each rune, and
  the margin between first and second underneath.
- **The preview box now holds two clouds.** The drawing in teal, the nearest
  rune in gold, in the same normalised frame — the gap you see is the distance
  the board reports. Position and scale are already divided out, and rotation
  deliberately is not (canon rule 6), so a reversed rune looks reversed here.
- **`CONFIDENT_MARGIN` is ours and lives in `debug.rs`.** `classify` is
  deliberately unthresholded because how close is close enough is a question
  about magic (§2.6), so the only judgement on screen is a word beside a number,
  parked in the overlay rather than smuggled into core.
- **The recorder now writes a file that loads, not a fragment that pastes.**
  Before, a traced rune could not be *matched* until it had been pasted into
  `templates.ron`, named, and the app rebuilt — which meant the tool that exists
  to unblock the recogniser could not feed it. `record` now emits a complete,
  valid template file and **appends** to it, so several runes can be traced in
  one sitting; the overlay rereads it whenever its mtime changes. Trace, press
  record, and it is scored on the next frame. Pasting still works — the entries
  in the middle are exactly what `templates.ron` wants.
- Placeholder ids are numbered (`RENAME_ME_1`, `RENAME_ME_2`), because
  `templates::parse` rejects a duplicate id within a kind, and it is right to:
  two runes with one name means one of them can never win a match.
- `subject_ink` is one helper now, because the board, the preview and the
  inspector must be looking at the same gesture or their numbers describe
  different drawings.

*Reaches it from the panel (§4.8):* **`runes`** toggles the board. **`record`**
is what puts anything in it.

**Three bugs off the board's first screenshot:**

- **The default font has no box-drawing glyphs.** `─ — · × ° → █ ░ ± ² … ≥` all
  render as tofu, and always had — `no rings ▯ ink so far`, `dpi ▯2.00`,
  `half ▯640`. The overlay pulls in Bevy's default font and nothing else (`ui`'s
  own note says so), so **every string `debug.rs` prints is ASCII now**. Rule for
  anything added later: if it is not on a US keyboard, it is a box on screen.
- **One long value slid the whole board off the left edge.** The block is
  right-anchored, so its width is its longest line — and a RON parse error at
  `15:1` is a very long line. Header rows are short and one-per-fact now, and the
  error is not printed at all (below).
- **A stale `recorded-gesture.ron` broke the board with a parser dump.** The file
  on disk was the *old* recorder's bare fragment, which has no `version` line, so
  every frame reported `Unexpected missing field named 'version' in TemplateFile`
  — true, unreadable, and not actionable. It now says
  `unreadable - press record to rewrite it`, which is exactly what fixes it. And
  `record` **moves an uncarvable file to `.bak` before overwriting**: tracing a
  rune is the expensive half of this project, and the recorder must not eat one.

**Two more found by sweeping rather than by screenshot:**

- **The panel's own hints were tofu too.** `bar.rs`, `mod.rs` and `record.rs` all
  printed `— · ° → § − ⌘ ⇧ ⌫`, so `(⌘Z)` read as three boxes. Every string the
  shell puts on screen is ASCII now; `⌘Z` is `Cmd Z`. The one survivor is
  `record.rs`'s `BEGIN` marker, which lives inside the `.ron` file and never
  renders — changing it would orphan a file already written.
- **The cloud preview has always been drawn underneath the tool panel.** The
  panel is twenty-odd rows tall, so open it owns the entire right edge, and the
  preview box has sat at the top right since M4.6. The overlay now insets by the
  panel's width while it is open. `PANEL_RESERVE` **duplicates** `bar.rs`'s
  `EDGE + PANEL_W` rather than reading it: §0 says a change to real code that
  exists only for the overlay is the wrong change, and the cost of drift here is
  a box twenty pixels off, never a wrong answer.

**Two more off the second board screenshot, and one of them was the lie again:**

- **The readout was compiling a `Glyph` it built by hand.** §9 claimed the spell
  block ran the whole pad through `compile_all`; it did not. It assembled one
  glyph from the inspected ring and called `compile`, which meant `unnamed` was
  never set — so a ring covered in four marks reported *bare ring, this is an
  explosion*, the exact lie `Warning::Unreadable` exists to stop. It also could
  never report nesting or links, because rules 4, 5 and 6 are questions about
  several glyphs and `compile` only ever sees one. It calls
  `assembly::glyphs` + `compile_all` now and indexes by ring slot — the two lists
  are index-aligned, one glyph per ring.
- **The font fix belonged in one place, not thirty.** Replacing `—` by hand in
  every string missed the ones core owns: `Warning`'s own `Display` writes `—`,
  so `bare ring [] this is an explosion` still had two boxes in it. `plain()` now
  flattens a whole readout at the point text becomes pixels, and anything it does
  not know becomes `?` rather than a box, so the next unforeseen glyph reports
  itself. Sanitising in the shell rather than in core is the same call §4.4 makes
  about pixels: which glyphs a font has is a fact about this shell, and core is
  entitled to write a proper dash in a `Display` impl.

**Light had to change the room, not just draw bright dots.**

The paper is already pale, so *more pale* reads as nothing — a light spell that
only brightens some parcels is a light spell you cannot see. So the **desk**
darkens as one takes hold and comes back as it spends: not the paper, because
dimming that would hide the ink, and darkening what is *around* the drawing is
what makes light read as light without taking the drawing away. Dimming is
quicker than recovery on purpose — a flare should feel sudden and its absence
should feel like your eyes adjusting. **Ours** (§2.6): canon says light
"manifests magic as light" and stops there.

**Sigils are placed like everything else, and the guide is placed at all.**

The element buttons used to stamp at the middle of the pad at a fixed size — a
different interaction from every other tool on the panel, for no reason anyone
could give. `Shape::Mark("fire")` goes through the same pick-then-place path as
a ring or an arc, and a drag sizes it.

The guide is now a click: `Shape::Guide` puts the lesson where you point, and
there is only ever **one** — clicking again moves it rather than adding a
second, because two half-finished lessons on one page would be two sets of ghost
strokes with no way to tell which step belonged to which. It also watches the
ring near *the lesson* rather than near the origin.

Removing `Toggle::Guide` and `Command::Element` afterwards was the point of
listening to the dead-code warning rather than silencing it: both had become
unreachable, and a leftover flag beside `tutor.placed` is two ways to say one
thing, waiting to disagree.

**The water orb, and why it is the clearest demonstration in the app.**

Four arrows pointing at the middle, water sigil in the centre. Canon's
`AllInward` — "manifests only inside the ring" — and it is the *arrangement*
doing every bit of the work: the same water sigil with the arrows reversed is a
fountain. Sigil says what, signs say how, ring says when. `stamp::seal` takes
`inward` and the preset table carries it, so the panel can lay either one down
and the difference is visible on the paper before it fires.

**M8.2 and M8.3: the shell reaches the browser.**

`webgl2` is added for the wasm target only — **not** WebGPU, which is still
behind a flag in most browsers, and a page that needs a flag is not a page you
can send to a friend. `web/index.html` is deliberately bare, because the app
draws every readout itself: a desk, a canvas, and a loading note that replaces
itself with the actual error if the module fails to start. A blank brown page
with no explanation is the worst way to discover a stale build.

`touch-action: none` and `user-scalable=no` are not decoration. Without them a
drag becomes a scroll and a two-finger rest becomes a zoom, and the pad cannot
be drawn on at all.

`./web.sh` says the thing everyone forgets: `cargo build --target wasm32` makes
a `.wasm` a browser **cannot load on its own**, and `wasm-bindgen` writes the
glue that can. It refuses loudly when `wasm-bindgen-cli` is missing rather than
producing a page that silently does nothing.

**The filesystem, answered rather than hidden.** `std::fs` *compiles* for
`wasm32-unknown-unknown` and fails at runtime — it builds, it looks supported,
and it quietly does nothing, which is the worst of the three options. So
`record` and the rune-file watcher are `cfg`-gated and the web build says
outright: **"recording needs the desktop build - the browser has no files"**.
The built-in shapes are compiled in, so the web build still *recognises*
everything the desktop one does; it just cannot pick up a rune traced since it
started. Tracing is a desktop job anyway — the reference is open beside the app.

**M8.1: core really does compile to wasm, and nothing had ever checked.**

§4.1 has claimed since M0 that `magic-core` compiles to
`wasm32-unknown-unknown` untouched — no Bevy, no wgpu, no `std::fs`, no threads,
no clock. It was never once tested, and it held on the first attempt. Every
refusal along the way is what paid for it: `Catalog::parse`, `templates::parse`,
`ReactionBook::parse` and `Materials::parse` all take **strings** rather than
paths; `Sim::elapsed` is derived from a tick count and not a wall clock; `swirl`
is a hashed pure function rather than an RNG; `Vec2` was written by hand; every
simulation path uses a sorted `Vec` or a `BTreeMap`. The two dependencies that
did come along, `ron` and `serde`, are pure Rust — which is exactly why §4.4 was
allowed to spend that budget.

**A sigil drawn on its own was firing, with no ring anywhere near it.**

Reported from a screenshot: a lone fire sigil showing `DISCHARGE - a blast,
summoning 0.2s left`. Its triangle is a **closed loop that fits a circle** well
enough to be found, and so are earth's chevron, light's square-and-diamond and
convergence's triangle.

`RingCandidate::activation` has asked "is this a ring at all" since M4.4 — and
`Ring` **threw the answer away**, so `compile` only ever asked *closed?* and
*neat?*. `Ring` carries `simple` now and `fire()` asks it **first**, because
"closed" and "neat" are meaningless questions about a triangle. Three regression
tests pin it, and the third is the one that matters: `a_real_ring_still_fires`,
because a fix that made everything inert would pass the other two and be
worthless.

This is the concept the whole engine rests on, stated back plainly: **sigil,
signs, ring — and no ring means no spell.** Ink without one is inert, which §3.3
already said and nothing enforced.

**Loose ink is named now, and still inert.** Canon rule 1 makes marks outside
every ring contribute nothing, and §3.3 makes them inert *rather than invalid* —
but inert should not mean invisible. Each one gets a faint caption: `fire (no
ring)`, or `unread`. Same clustering and same threshold as inside a ring,
deliberately, because a mark must not change its mind about what it is the
moment a ring is drawn round it.

**Tracing ends in the app rather than in a text editor.** `trace >` picks which
rune the next recording *is*, and `record` writes it under that name — the
`RENAME_ME_n` placeholder is now only the fallback. Re-tracing replaces the
earlier attempt, because re-tracing is how a shape gets better and keeping both
would leave the worse one competing. This is the workflow §2 always described: a
person traces, the app records, and the built-in reconstructions stand aside.

**An eraser.** `clear` and `wipe` are all-or-nothing and a seal is a dozen
strokes; one bad keystone should not cost the ring. Nearest stroke only, because
erasing more than you pointed at is what makes an eraser frightening.

**The compiler was unreachable from a drawing, and had been all along.**

`assembly` found rings and what they held. The recognizer could tell one shape
from another. The compiler knew what every sigil and sign does. **Nothing joined
them**, so every seal drawn by hand compiled to canon rule 9's discharge — and
`templates.ron` shipping empty meant there was nothing to recognise anyway.
Milestones M4 and M5 were each finished and the pair of them did nothing
together.

`shapes.rs` gives the five elements and three signs a geometry, and `naming.rs`
is the join. Three decisions in it are worth stating:

- **Which strokes are one mark.** Fire is four strokes and a column is two, so
  "one stroke, one mark" is wrong for almost everything. Single-link clustering
  on stroke centres, at a distance scaled by the ring — the parts of one mark
  are drawn touching, and two marks in a seal are drawn far enough apart to be
  legible as separate things.
- **Which mark is the sigil.** By the *template's kind*, never by position.
  §2.2 says outright that a sigil's location does not change its behaviour, so
  picking "the middle one" would invent a rule canon denies.
- **Where a sign sits.** `placement` from the ring's centre and `orientation`
  from the mark's own long axis, because §2.4 makes the tilt between those two
  the difference between reach and spin.

**A match has to win, and win clearly.** The first threshold was distance alone
and `ink_nobody_recognises_is_counted_rather_than_guessed` caught it: `$P` always
returns a *nearest* template, so "nearest, and near enough" cheerfully called a
scribble a fire sigil. Real ink is decisively nearer one shape; noise is roughly
as far from everything. So `NEAR_ENOUGH` is joined by `CLEAR_MARGIN`, and the
runner-up is what decides whether the winner won — the same question the
recogniser board has been printing since M4.7, promoted from a readout into the
decision.

**Duration is on screen.** `summoning - 3.4s left` under the seal, and every
running spell in the world block. The question a person has while watching one is
"is this about to stop", and the only answer used to be to keep watching.

**The guide followed the pointer, which made it useless** — the thing you are
trying to trace moved as you reached for it. A guide has to hold still or it is
not a guide.

**The element buttons stamp the sigil and then clear the override.** Forcing a
name would hide whether the recognizer actually read the drawing, and that is
now the interesting question rather than a formality.

**A stamped seal is useful for testing and useless for learning.**

`preset` puts a finished drawing on the paper without ever putting it in your
hand. `ui/tutor.rs` is the other half: the same seal, one stroke at a time, with
the next stroke shown faintly where it goes and the pad watched to see when you
have drawn it.

It teaches **whatever `preset >` has selected**, deliberately rather than
carrying its own list of lessons. A second list is a second thing to keep in
step, and the property worth having is that the button which stamps a seal and
the guide which teaches it cannot disagree — both read `PRESETS`, both draw with
`stamp::seal`.

There is **no "next" button**, because there is nothing to press: you have either
drawn the thing or you have not, and `Reading` already knows. The step you are on
is drawn bright, what you have finished stays faint — a guide that shows
everything at full strength is a picture rather than a lesson. The first step
centres on the pointer so a lesson starts where you are looking, and the ring it
watches is the one nearest the size being taught rather than simply the first, so
a stray loop elsewhere on the paper cannot hijack it.

**The last line is the one that matters:** *"done — and canon never asked for
that order. Any order draws the same seal."* §2.5 is explicit that canon never
constrains drawing order — an unclosed ring is a fully prepared spell, and half a
split seal is drawn with no complete ring at all — so ring, centre, keystones,
close is a *teaching* order and nothing more. Saying so is the difference between
a guide and a rule somebody invented.

**Drawing felt broken because the pad was being read eight times a frame.**

`find_rings` fits a circle to every stroke by Taubin, measures winding and
coverage on the survivors, then asks each ring what it contains; `compile_all`
follows. Doing that once is cheap. Doing it in the inspector, the fit line, the
ring captions, the recogniser board, the fit drawing, the auto-cast watcher and
`cast_pad` — each for itself, every frame — is not. At four hundred points that
is the whole difference between drawing feeling immediate and feeling broken.

`reading.rs` reads it once and everyone reads the answer, and it only recomputes
when the ink or the naming actually changed — remembering what the pad looked
like rather than trusting a `Changed` filter, which fires on any write. The
pixel thresholds moved there too, which is where §4.4's units rule always said
they belonged.

**The preset would not fire twice, and the cause was the watcher's memory.**

`preset` clears and re-places in the *same* frame, so the watcher never saw an
empty pad — and the new ring landed at the same centre and radius as the old
one, which is exactly how it decides two rings are the same ring. It matched a
seal already marked *fired*, the rising edge never happened, and nothing cast
until the pad was wiped by hand first. `undo`, `redo`, `clear`, `wipe`, `preset`
and the element buttons all clear that memory now.

**`sigil >` was working; you could not see it.** The hint line showed *either*
the hovered button's description *or* the last result, and cycling a list is
precisely when you are still hovering the button that produced the answer you
wanted to read. Two lines now, always: what the button does, and what just
happened.

**A spell that fires once is a firework, not a fountain.**

`Sim::channels` — a cast registers a *running* spell that keeps summoning tick by
tick. Canon points straight at it: long-duration water spells **collect** rather
than create, which is a claim about a spell that runs for a while, and nothing in
the model was long about anything. Duration is rule 8 twice over — `Fleeting`
gets a fifth of the time, and among seals that hold, a neater ring runs longer.

Two details carry it. The **rate** is what the seal decides and `dt` only decides
how finely it is chopped, so halving the timestep gives twice as many casts of
half the size — the same water, which §4.3 requires of anything on a clock. And
there is **one channel per seal, not per press**: casting the same seal again
refreshes it rather than stacking, which is what stops a held button becoming an
ocean.

**Walls became a rule rather than a fact.** `SimRules.walls` on, nothing escapes
and mass is conserved exactly — which is what makes every M6.8 property testable.
Off, `Field::spill` drops what leaves and the mass total visibly *falls* instead
of the field quietly keeping it. Both honest; only one measurable, so walls are
the default and `walls` on the panel turns them off.

**Prepared presets.** Every preset has an open twin — ring left with a gap,
everything else drawn. Canon rule 2 as a tool: "leaving a gap prepares a spell to
be fired later by closing it". They land inert and the last stroke is yours,
which is the moment the whole engine is built around and the one thing a finished
preset can never show you.

**Five seals, one per element**, and an `element` section that names a seal
outright — hunting for `water` in a list of thirty-four is not a thing to make
somebody do. `water_orb` is water plus four levitation signs, taken from the
community spell simulator's own sample and agreeing with what the wiki says
levitation does: floats the target, often shaping it into a sphere **when
balanced**. Recorded as `Inferred`, because the arrangement is reconstructed
rather than quoted.

One correction rode along with it: canon's primary tetrad is fire, water, earth
and air, and **light is a fire *variant*, not a fifth element**. The wiki is clear
and the popular summaries are not. It is on the panel because it is the fifth
thing anyone wants to try.

**A preset seal with nothing in the middle was a real bug, not a shortcut.**

Every seal in the source has a sigil at its centre, and `preset` was laying down
a ring with arrows around an empty hole — a drawing nobody would recognise as a
seal, wrong before the compiler had said a word. `stamp::seal` puts a mark there.

Deliberately **not** a small ring: the ring search would find it and read it as
canon rule 4's nesting, which would change what the seal compiles to. A
three-sided mark inside a cross cannot be mistaken for one. It is a stand-in and
not a traced rune — §2 still holds, the shapes belong to the manga — but the
centre has to be occupied.

The seal is one function so the button and the **hover preview** draw the same
thing. Hovering `preset` now paints the whole seal on the paper in ghost purple,
which matters because that button's entire job is "put a complicated drawing
here" and the hint line can only say its name. Calling the same function is what
stops a preview from lying.

**The room full of air turned the world overlay into a grid of boxes.**

`fill_air` gave every one of 576 cells some mass, and the overlay shades any cell
with mass in it, so all of them lit up and the drawing was lost inside. It shades
only cells above *a breath of air* now, and shades by how packed they are, so a
pool reads as a pool rather than as a uniform stencil.

Two smaller ones from the same screenshot: `repetition - MOVES ` with nothing
after it, because guidance, obliviation, doorways and repetition act on no
substance at all and the sentence stopped halfway; and `preset` still advising
"press cast" with `auto` on, which is advice from two versions ago. And `pour`
drops under the pointer now rather than at the world origin, because two
substances could otherwise only ever be made to meet in one place.

**Water had no surface, and the reason was a cancelling gradient.**

Buoyancy and cohesion between them give water something that falls and holds
together, and neither says a liquid is nearly **incompressible**. Without that
term water collapses into whichever cell is lowest and sits there as a dot — no
level, no heap, no pool. `MaterialDef::stiffness` is the term.

The first attempt moved a pile **0.55 pixels in ninety ticks**, and the failure
was a modelling error rather than a tuning miss: **a pressure gradient sampled at
cell centres cancels for any symmetric blob.** A lone overdense cell surrounded
by empty ones pushes equally in all four directions, so the sum is exactly zero.

`Cell` gained a **`centroid`** — the mass-weighted mean *position*, which is not
the cell's geometric centre — and pressure now pushes each parcel away from where
its cell's mass actually is. That term vanishes in an even pool, because there a
parcel already *is* where the mass is, and bites hardest in a heap. The gradient
term stays alongside it, because that is what levels a pool once it has one.

`coalesce` had to give ground for it: merging every same-substance parcel in a
cell down to *one* left pressure with nothing to push apart, since a single
parcel is always exactly at its own centroid. It keeps **four per cell** now —
still a bounded count, still no runaway, but a gradient survives. And pressure
only engages *above* the rest density, so a thin scatter of droplets does not
fly apart.

**`momentum NaN, NaN`, and 36,756 parcels holding 400 units of mass.**

Both the same bug. Every reaction emits *new* product parcels and merges
nothing, so a world where anything is reacting gains parcels every tick and
loses none. After nine thousand ticks the average parcel weighed a hundredth of
a unit, which is not a lump of anything — and a weighted mean over tens of
thousands of near-zero masses is where the `NaN` came from. Then it spread,
because every sum a `NaN` enters comes out `NaN`.

`Field::coalesce` merges parcels of one substance sharing one cell. **Exact, not
approximate**: mass adds, momentum adds, heat adds, and velocity, position and
temperature become the mass-weighted means — `merging_conserves_momentum_and_heat`
pins all three. A parcel carrying an anchor or a countdown is left alone, because
both are facts about *that* parcel and merging would discard them silently.

`Field::scrub` catches a ruined parcel instead of letting it spread, and `Sim`
counts what it caught. §4.7 forbids panics, and a `NaN` is the *silent* version
of a panic — the only honest thing left is to notice it and say so on screen
rather than have the field quietly heal itself.

Written the second time without an index loop: the first version walked indices
and kept a `merged` flag per parcel, indexing two collections by one counter.
Moving the parcels out and pushing survivors says the same thing with no index
anywhere.

**The overlay was hiding the two things a person actually needs.**

`debug_overlay` is off by default now, and that is only safe because the ring
caption and the world stopped being *debug*. "What will this seal do" and "what
is in the world" are the questions you have **while drawing**; putting them
behind F1 meant turning the measurements off left the app unusable rather than
clean. They ride `spell` and `world` now and still vanish with `debug.rs`, which
is what §0 asks.

With the measurements off the caption is the answer and nothing else —
`aeriforms - MAKES air` over `ACTIVE - circuit closed`. Turn them on and it grows
its stroke ids, radius and rms back.

**The hint line was being missed, and it was saying the wrong thing.**

It hung below the slab, which was fine at one column and hopeless at two — the
panel now reaches most of the way down the window, so the hint was landing in the
last few pixels of screen. It runs along the bottom edge now, at 16px, bright, on
a dark plate, because pale text on parchment is not text.

And it answers the actual question. `sigil >` used to say `sigil 7/34: aeriforms`
— which button you pressed, and nothing about the magic. `describe_sigil` reads
the catalogue instead: `aeriforms - MAKES air (cannot move it)`,
`wind - MOVES air (cannot create it)`. The **refusal is spelled out** rather than
left as the absence of a word, because §3.2's makes-versus-moves is the most
load-bearing fact in the engine. `spell >` prints the fixture's own effect line
and the signs it is built from.

**"It doesn't fire when the ring is closed."** It did. The world was a vacuum.

Canon rule 9 makes a bare ring an explosion, and an explosion is **energy** — it
shoves and heats what is already there and creates nothing (§3.2 would be a lie
otherwise). In an empty field that is, correctly, *nothing at all*. So the
compiler said `Active`, the cast reported `Discharged`, and the screen stayed
blank, because a room with no air in it is not a room.

`Simulation::fill_air` seeds still air at room temperature across the field at
startup and after `empty`. One change, three fixes: a discharge has something to
throw, a wind spell can **find** the air it is forbidden from creating, and fire
has fuel — so `fan` and `burnout` become visible rather than theoretical.

It cost two performance fixes that were bugs waiting anyway. `react` was building
a `SubstanceId` — and therefore a `String` — per input, per cell, per rule, per
tick, purely to run an equality test; with a room full of air that is millions of
allocations a second, and it compares `&str` now. And each rule now bails before
bucketing if the field holds none of its inputs. `fan` was also retuned from
`rate: 1.0, heat: 120` to `0.2, 60`: with air everywhere the old numbers turned
every spark into a firestorm that ate the room.

**"The water doesn't feel like water, the fire doesn't feel like fire."** Both
true, and both the same bug: every parcel fell at the same rate under the same
drag, so water and flame were one simulation wearing two colours. Colour was
never going to fix it.

**The formula is Archimedes, on a density that varies by the ideal gas law.**

```
a    = g * (1 - rho_fluid / rho_body)      buoyancy
rho(T) = rho_ref * T_ref / T               ideal gas, ABSOLUTE temperature
```

`materials.ron` gives flame `density: 1.2` — **the same as air** — and nothing
anywhere says "flame goes up". At 800 °C flame is `1073 K` against a room's
`293 K`, so it is a third of the room's density, the bracket goes to about
`-2.6`, and it climbs at two and a half gravities. Cool it and it stops climbing
**on its own**, because the cause went away. That is the whole reason to model
it rather than script it.

Steam is the argument's other half: `density: 0.6`, because a water molecule is
lighter than the nitrogen it displaces. **Steam rises after it has gone cold**,
which no heat rule could produce. Most of the numbers are real — air 1.2, water
1000, ice 917 (which is why it floats), stone 2600, sand 1600 — and a real number
beats a tuned one every time.

**Phase is what buoyancy alone cannot do.** Water at 1000 kg/m³ still behaved
like heavy air until `cohesion` existed: how hard a parcel pulls toward the mean
flow of its cell, which `Field::settle` was already computing. Water at 7.0 moves
as a body and pools; flame at 0.25 disperses. Same loop, opposite feel.

**Fire flickers, and it is still deterministic.** §4.3 forbids randomness
outright, so `swirl` is a *pure function* of quantised position and tick, hashed
— same seal, same frame, same flicker, every run and every machine. Quantised so
neighbouring parcels swirl together rather than each doing its own thing, which
is the difference between a flame and static. Turbulence scales with heat, so a
cooling flame stops flickering for the same reason it stops rising.

**`Phase::Radiant` is an honest hole made explicit.** Light neither falls nor
floats, and giving it a weight would have been worse than saying so.

**Three old tests broke immediately, and that was the good news.** They used
`air` as a stand-in for "a thing", and air at room temperature is now neutrally
buoyant *in air* and correctly does not fall. `drop_of` makes water now.

**Two bugs the new tests caught:**

- **A stone bounced forever.** `Field::confine` reflected velocity perfectly, and
  a wall with restitution `1.0` is a trampoline — nothing came to rest, so
  nothing ever settled. Walls keep 25% of the impact speed now.
- **A test measured a bounce.** `a_flame_that_cools_stops_rising` compared one
  flame's climb before and after cooling, but the hot one had already hit the
  ceiling, where the wall turned its velocity negative and made "slower than
  before" quietly false. Two fields now, hot against cold, comparing height.

**`burnout` closes the fire loop.** Flame below 180 °C becomes smoke, so spent
fire does not hang about as cold flame forever — which looked exactly as wrong as
it sounds.

**Four more marks in `place`**, and one of them closes a gap §9 has been
carrying since M5.2:

- **`bar`** — a straight keystone. Length is power, direction is aim (§2.4), and
  no shape to distract from it: four equal bars sum to zero drift, lengthening
  one steers the seal.
- **`triangle`** — three-sided, after canon's note that whorling wind is
  three-sided. A stand-in for that shape, not a claim to be it.
- **`spiral`** — ink that turns two and a half times while covering one. The ring
  search must *reject* it (`is_simple` fails), and this is how to produce that
  case without a steady hand.
- **`link`** — drag from one ring to another. **Canon rule 5 had no way in from
  the panel at all**; it was reachable only from tests, which §9 recorded as the
  largest remaining gap after M5.7. A link is drawn from press to release rather
  than out from a centre, because it joins two seals and neither end is the
  middle of anything.

*Reaches it from the panel (§4.8):* **`preset`** lays a whole seal down — ring
plus four equal column signs — and **names** it, so `cast` fires without anything
being drawn. **`preset >`** chooses which: flamespout, watershot, raincleaver,
everlasting, windrider, mistveil. And canon settles where a jet goes: identical
column signs give "a balanced spell that shoots straight up", one longer sign
"makes the spell shoot off to the side" — so a balanced seal now fires **up** in
a narrow fan instead of in a ring, and a leaning one fires the way it leans.

**The effect was invisible, and the reason was one word: outline.**

`gizmos.circle_2d` draws a **hairline ring**, not a disc. A field of parcels was
a field of one-pixel circles on parchment — technically drawn, practically not
there, and no amount of colour-picking would have fixed it. Parcels are sprites
now, pooled and reused the way `RingLabel` is, and `clear_parcels` drops the pool
when the overlay hides because a sprite is not a gizmo and does not stop on its
own.

Colour carries two facts at once: **hue is the substance, brightness is the
heat**. The first version ramped one blue-to-orange scale by temperature alone,
which made every substance look identical the moment it was hot — a hot stone and
a flame were the same colour. Mixing each substance's own hue toward white keeps
both readable. `substance_color` is a `match` on a name in a *shell* file, which
is the right place for it: what colour water is on this screen is not a fact
about magic (§4.2), and an unknown substance gets a plain grey rather than a
guess, so adding one to `reactions.ron` shows up as something visible.

**Thirty-three more spells, and the honest holes kept as holes.**
`spells.ron` went from 13 fixtures to 46. `SpellDef` gained `confidence` and
`glaives`:

- `Canon` — composition documented. Raincleaver's linked seals, memory erasure
  with its three glaives, the horse sigil pulling a cart, everlasting's
  repetition, flower's five modifier symbols.
- `Inferred` — reconstructed from signs the wiki *does* describe, never from a
  glyph anyone decoded. Windrider, mistveil, sandserpent, coilspring.
- `Unknown` — **named in the source and nothing else**. Gryphon and pegasus are
  in the catalogue with no signs, no effect beyond "nobody has seen it", and
  that is the entry. `a_spell_of_unknown_composition_lists_no_signs` makes §2's
  rule mechanical: writing signs on an undecoded spell would compile, look
  right, and be fiction.
- `only_the_two_canon_spells_carry_glaives` pins the other direction — glaives
  are nearly forgotten since the Day of the Pact, so a third spell growing them
  is a curation mistake rather than a discovery.

The loader's own cross-check did the rest: all thirty-three name only sigils and
signs that exist, or the catalogue would not have loaded.

*Reaches it from the panel (§4.8):* `spell >` walks all 46. The world block names
what the pad is currently set to and what `pour` will drop, because with
`templates.ron` empty the naming override is the *only* thing making a seal
compile to anything but a discharge — and a person who forgets they left it on
fire has no other way to find out.

**Where M7 landed** — two element instances finally meet:

```
crates/magic-core/the-magic-assets/reactions.ron   10 rules, every one ours
crates/magic-core/src/sim/reaction.rs              ReactionBook, react
```

- **The whole module is ours** (§2.6), and it says so at the top. Canon has no
  reactions — spells are discrete effects with no mechanism, and the wiki never
  says wind and water make a vortex. So the rules are a `.ron` file (§4.4) and
  arguing with one costs an edit rather than a recompile.
- **What is *not* ours is what the rules must obey.** §3.2's capability model
  only means something if matter is conserved, so a rule's output shares must
  total `1.0` and `ReactionBook::parse` **refuses the file otherwise** — with a
  message saying whether it would create or destroy mass. A rule that quietly
  mints matter would make every conservation property in `sim` a lie, so it is
  caught at load rather than debugged at runtime.
- **Momentum rides on the mass.** Products inherit the pooled momentum of what
  was consumed, split by share, so two parcels meeting head-on leave as one going
  nowhere. Heat is *accounted*, not conserved: boiling takes 540 degrees out of
  the world and freezing gives 80 back, which is what makes a kettle plateau and
  a pond hold at zero.
- **One input is a phase change, two or more is a reaction.** Same struct, same
  loop, and the distinction is a `len()` rather than a second concept.
- **Reactions happen to a *place*.** Parcels are bucketed by cell — a `Vec` of
  `Vec<usize>` filled in parcel order, never a map (§4.3) — and a rule is tested
  against the *cell's* temperature, not any one parcel's. The scarcer input sets
  the pace, so one drop of water cannot consume a bonfire.
- **Motion runs before reaction**, because substances have to be moved into the
  same cell before they can be said to have met there. Reacting first would let a
  parcel react with wherever it used to be.
- **The rules read the same dialect as the rest of the assets.** The file would
  not parse at first because `catalog.rs` enables RON's `implicit_some` and
  `reaction.rs` did not, so `min_temp: 100.0` was a syntax error while
  `variant_of: "fire"` next door was fine. One dialect across `the-magic-assets`,
  or every file is its own puzzle.
- **A test that passed for the wrong reason.**
  `substances_in_different_cells_do_not_meet` put water and flame in far-apart
  cells at 200 degrees and checked no steam appeared. Steam appeared — the water
  simply *boiled*, which is a phase change and nothing to do with meeting. Run at
  80 degrees it measures what it claims to.

*Reaches it from the panel (§4.8):* **`pour`** drops a ring of one substance in
the middle of the world and **`pour >`** chooses which — water, flame, air, ice,
stone, steam, each at a temperature that makes something happen. Two clicks from
a cold start to a reaction. The world block in the inspector grew three lines:
what the world **holds** by substance, how many **rules** are loaded and over
which substances, and what is **reacting** this tick — rule by rule, with the
mass turned over and the heat released.

**Where M6 landed** — a compiled spell finally does something:

```
crates/magic-core/src/sim/
  vec2.rs    a 2-vector, written rather than depended on
  parcel.rs  SubstanceId, Parcel, Anchor, AMBIENT
  field.rs   Cell, Field - the grid, and the only place mass moves
  step.rs    SimRules, step - motion, heat, repetition, expiry
  cast.rs    CastRules, cast - Spell -> matter
  mod.rs     Sim - a world, its rules, and how far it has run
apps/canvas/src/sim.rs   the shell's copy: FixedUpdate at 60Hz, cast_pad
```

- **M6.1–M6.2, the nouns.** `Vec2` is written rather than pulled in: `glam`
  would satisfy §4.1 fine, but sixty lines against a third dependency in a crate
  that has two is not a trade worth making. `Point` was not the answer — it
  carries `stroke_id`, which is a fact about ink, and a parcel of steam was never
  drawn. `Field::cell_at` checks the sign **before** casting to `usize`, because
  casting `-1.0` saturates to `0` in Rust and would file something off the left
  edge into the first column.
- **A cell is a summary, never authored.** Density, temperature and flow are all
  recomputed by `settle` from the parcels; writing one directly would mean the
  parcel it should have come from is missing. Mass-weighted, so a speck cannot
  outvote a boulder — and `settle` is idempotent, which is a test, because it
  runs every frame.
- **M6.3, determinism, and the M5.6 lesson applied in advance.** Fixed timestep,
  no clock, no randomness, parcels in a `Vec` walked in slice order. `step` reads
  every parcel's cell into a list *before* mutating anything, so a parcel
  changing its own cell mid-pass cannot make the answer depend on where it
  happened to sit in the list. `simulation_state_at_frame_n_is_identical_across_runs`
  is §5's own property, and it passes.
- **M6.4, and the reason §3.2 was worth encoding.** `Demand.must_find` becomes
  `Field::take`, which drains the nearest parcels first and returns what it
  actually got. Wind moves air but cannot create it, so a wind seal in an empty
  room returns `NothingFound` and raises nothing at all. Aeriforms creates air,
  so it raises its own. The audit walks **every elemental sigil in the
  catalogue** and asserts mass appears from nothing only where the wiki says it
  may — a capability model that nothing enforces is just documentation.
- **Rule 9 is energy, not matter.** A discharge creates nothing: it shoves what
  is in reach outward and heats it, and in an empty room it does nothing visible,
  which is the honest answer rather than a puff of invented smoke. Four tests.
- **Canon's four region cases decide where parcels appear.** `AllSameSide` fans
  them along the heading, `AllInward` puts them inside the ring, `AllOutward`
  outside it, `Opposed` on the ring itself as canon's floating drops. Computed by
  M1R's `arrangement`, unused until now.
- **§2.4's spin/reach split becomes velocity.** `reach` pushes outward, `spin`
  pushes along the tangent, and the two always sum to one because `Spin`
  guarantees it. Tilt trading reach for rotation is visible on screen.
- **M6.5, repetition is a simulation rule outright.** Canon: "continuously resets
  affected objects to the state they held when the spell took hold, temperature
  included". `Parcel::anchor` is a snapshot of exactly the three fields that can
  drift, and a repetition-driven cast sets it on everything it raises. It is a
  *pull*, not a snap, because canon's own word for the effect is spring-like —
  and the stiffness had to go from 6.0 to 20.0 after a test caught a held parcel
  sagging 19px under gravity. A sag is not a reset.
- **M6.6, lifetime, and canon rule 8.** `Fleeting` is the wiki's own word for a
  ring too rough to hold, so its parcels last a tenth as long as an `Active`
  seal's rather than merely less.
- **M6.8, conservation, and one design decision it forced.** A parcel that
  reaches the edge is turned back rather than deleted — **ours** (§2.6), and the
  alternative was worse: vanishing loses mass silently, which would make every
  conservation property untestable. So the world has walls, and every test that
  matters is written against `Field::mass`.
- **What is deliberately not modelled yet:** no reaction between substances, no
  phase change, no density-driven buoyancy. That is M7, and inventing it early
  would be exactly the guess §2 warns against.

*Reaches it from the panel (§4.8):* a new **cast** section — **`cast`** compiles
every seal on the pad through `compile_all` and fires it, **`run`** starts and
stops the world, **`tick`** advances exactly one step so a frame can be read
rather than watched, **`empty`** clears the world without touching the ink. In
`view`, **`world`** toggles the overlay: the grid, every cell holding mass, and
each parcel as a dot sized by mass and coloured by heat, with a whisker showing
where it is going. The inspector grew a `world` block — state, tick, simulated
seconds, parcel count, total mass, total heat, momentum, hottest parcel, and what
the last cast did.

`docs/m6.1-first-parcel.md` is the written guide to the first task, kept as
written: thirty steps, every symbol defined.

*(was M5.7 — the two rules that had no way in from the panel)* Nesting and links compile correctly and can only be reached from tests,
because nothing on the pad detects a ring inside a ring or a line between two
rings. That is `assembly`'s job, not the compiler's. Then M5.8, the recorder.

**Canon audit, taken at M5.3.** Every rule in §2 checked against what the code
actually reads. Four gaps were found and three are now closed: glaives had no
type at all, rule 8's absolute size was being conflated with §2.2's sigil ratio,
and sign *count* was never read separately from sign *size*. The fourth — the
three effects of a decorative sigil (sculpt, target, restrict) — is recorded in
`sigils.ron` and still unread, and stays open because nothing can yet name a
decorative sigil on the pad.

Nothing in M5 is blocked on `templates.ron`. The compiler takes a `Glyph` —
ids, angles and sizes — and never sees ink, so every canon rule can be built
and tested before a single rune shape is traced. What the empty catalogue
blocks is the *shell*: nothing on the pad can be named a sigil yet, which is
M5.7's problem and still needs a recorder.

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

**Where M5.1 landed** — the compiler, and the one thing it refuses to do.

*Reaches it from the panel (§4.8):* **`sign`** places a keystone — drag aims it
and sets its length, which is everything `balance` and `spin` read. **`spell`**
toggles the compiled-spell readout. The readout itself lands with M5.7; the
tool and the flag are in place so the wiring has somewhere to arrive.

- `compiler.rs` — `compile(glyph, catalog, rules) -> Spell`. **Total: there is
  no error type.** Canon calls almost nothing invalid. An unbalanced seal fires
  sideways, an asymmetric one fires unstably, an open one is a *prepared* spell
  (rule 2) rather than a broken one — so every glyph compiles and everything
  wrong with it rides along as a `Warning` carrying the number behind it. A
  shell can say "leans 0.4 toward 130°"; it never has to say "invalid".
- **Rule 9 is a first-class case, not a fallback.** A bare ring compiles to
  `Driver::Discharge`, fires, and reports full intensity — there is nothing
  smaller than the ring for an explosion to be a fraction of. Four tests guard
  it, because "nothing in the ring" reading as "nothing to do" is exactly the
  bug canon warns about. A ring holding signs but no driver is the same
  discharge with the signs shaping the blast, and `NoDriver` says which case it
  was.
- `Firing` is deliberately **not** `Activation`. That enum answers a question
  about ink — is this shape a ring at all — and is settled before a `Glyph`
  exists. `Firing` answers a question about a spell from the two facts a `Ring`
  carries, and asks them in canon's order: structure before craft, so an open
  ring is never judged on how neatly it was inked.
- **`reversible` became `Option<bool>`, and that was a real bug.** Three signs
  in the whole catalogue state it; `#[serde(default)]` made the other 41 read
  as *not reversible*, which is not what the data says — it is what a missing
  field says. §2.3's own table settles it by class (directional and
  semi-directional invert, non-directional cannot — "no front to point"), so
  `Catalog::is_reversible` derives it and the field is an override. `None` for
  an asymmetric or unclassified sign, and the compiler warns only on a stated
  `false`: guessing `false` would silently discard a legal spell.
- **`Glyph` gained `sigil_extent`.** §3.1 records twice that scale must be
  captured before it is divided out, and the glyph is downstream of both places
  that capture it. Intensity is `sigil_extent / ring.radius()` — the wiki's own
  "size of a sigil in relation to the ring" — and `IntensityUnmeasured` says so
  out loud rather than reporting a zero nobody drew.
- Three predicates moved into `Catalog` where they belong: `is_directional`
  (the one `balance` and `spin` have been asking callers for), `is_region`, and
  `region_signs`. Region is found by **having an `arrangements` block**, not by
  matching the id `"region"`, so a second region-like sign would need data and
  nothing else (§4.4).
- Capabilities reach the spell as `Option<Capabilities>` — `None` for a
  discharge, which has no substance to conserve, and for a substitute, whose
  capabilities the wiki genuinely does not state. Tests pin the two that matter:
  wind moves air and cannot create it, aeriforms creates air and cannot move it.

**Where M5.2 landed** — the three rules one glyph cannot answer:

- `compile_all(glyphs, catalog, rules)`. `compile` is untouched and still the
  whole story for one seal; this runs it per glyph and then applies rules 4, 5
  and 6 **in that order**, because a seal held shut by an outer ring has nothing
  left to amplify or cancel.
- **Rule 4** — `gate_nesting` walks *outward* from each glyph, so a chain three
  deep costs one pass per glyph and needs no ordering of the input. A step cap
  turns a malformed `parent` chain into `NestingCycle` rather than a hang (§4.7).
- **Rule 5** — `amplification(n) = sqrt(n)` per seal, so `n` linked twins total
  `n^1.5`. **Ours**, and the shape is forced: canon says linked seals beat "a
  single large spell that took up the same amount of space", which is false for
  anything linear. Two are worth 2.8, four are worth 8.
- **Links are mutual even when recorded one way.** A drawn line has no
  direction, and requiring both sides to list each other would make the answer
  depend on which glyph was assembled first (§4.3).
- **Rule 6 needed a denominator, and finding that out was the bug.** `inverted`
  alone made any two seals sharing an upright reversible sign read as twins. A
  seal is a twin only if *every* sign that could flip did — so `Spell` also
  carries `reversible`, the effective signs the catalogue says can be mirrored
  at all. A mirrored non-directional sign stays out of both lists: it is a flag
  nobody can act on, already reported as `NotReversible`, and it must not make a
  seal look like somebody's twin.
- Cancellation is asked **before** amplification, because a twin satisfies
  `same_spell` too and would otherwise read as a copy.
- `Spell::strength()` is the one number the simulation will want: zero when the
  seal is not firing or a twin cancelled it, otherwise intensity times whatever
  its linked copies add.

*Reaches it from the panel (§4.8):* **nothing yet.** Nesting and links have no
tool, so both rules are currently only reachable from tests. That is the gap
M5.7 has to close, and it is now the largest one.

**Where M5.3–M5.6 landed** — and the canon audit that came with them:

- **M5.3, sign requirements.** Three relations in the data, three different
  strengths, and the difference is the point: `requires` is hard and every one
  must be present (billow "needs collection to gather the material first", so
  without it billow contributes nothing); `requires_one_of` is hard but any one
  will do (enlarge needs selection *or* diamond, because those decide self
  versus nearby); `pairs_with` is soft — the wiki has only ever *seen* the two
  together, so the sign alone is unrecorded rather than broken, and it is
  reported and kept.
- **Glaives now exist.** §3.3 called for them and there was no type. `Glaive` is
  its own struct with its own list on `Glyph`, deliberately not a `SignId` —
  folding it into `signs` would have put it into `balance`, `spin` and the
  symmetry classification, where canon never puts it. `Spell::embedding` reads
  total glaive size against the ring, and nothing filters glaives by position,
  because they are rule 1's one exception.
- **Rule 8 and §2.2 were being conflated.** `intensity` is a *ratio* — sigil
  against ring. `scale` is the seal's outright size — "larger seals are more
  powerful than smaller ones". Two different canon claims, now two fields. A big
  seal with a small sigil is powerful and unfocused; a small seal with a filling
  sigil is focused and weak.
- **Count is a third knob.** "The amount of signs will affect the range or
  quantity of magic generated" is separate from size being power.
  `Spell::sign_count` reads it; `Balance::power` only ever summed the second.
- **M5.4, conservation.** `Demand` turns the capability model into a rule the
  simulation can enforce: what substance the spell acts on, and whether it must
  *find* it. A test asserts every elemental sigil that cannot create names what
  it needs — a conservation rule with no substance attached is unenforceable.
  Non-elemental sigils are excluded, because guidance, obliviation and doorways
  act on no substance at all and an empty list is the right answer for them.
- **M5.6 found a real determinism bug.** `balance` summed sign pushes in arrival
  order, and float addition is not associative — the same seal with its signs
  listed in two orders gave `0.051757645` against `0.051757623`. §3.3 is explicit
  that stroke order is never an input, so `balance` and `spin` now sort their
  contributions before summing. The property test that caught it is the one from
  §5's table, and it would never have shown up in an example-based test.

*Reaches it from the panel (§4.8):* **`glaive`** places a claw. **`spell`** now
renders — the inspector carries driver, firing, strength, intensity,
amplification, scale, sign count, embedding, symmetry, region, balance, spin,
what the spell must find, and every warning. `debug.rs` decides nothing: it
builds a `Glyph`, calls `compile`, and prints what comes back.

Every seal on the pad currently compiles as rule 9's **discharge**, and that is
the honest answer rather than a placeholder — `templates.ron` is empty, so
nothing can be *named* a sigil yet. The moment a rune is traced the block starts
saying something different without changing.

**Where M5.7 and M5.8 landed** — the pad becomes glyphs, and runes get out:

- `assembly::nesting` reads rule 4's structure off **geometry**, because there
  is no other way to draw it: a ring is nested in the *smallest* ring that
  encloses it, which is what makes a three-deep stack resolve to a chain rather
  than to everything pointing at the outermost. Touching does not count — two
  rings that graze are two seals, and rule 5 is what joins those.
- `assembly::links` finds rule 5's joining line: ink belonging to neither ring
  that comes within tolerance of both. A ring's own strokes can never link it.
  Pairs come back lower-index-first so the answer does not depend on which ring
  was found first (§4.3).
- `assembly::glyphs` is the last step before magic — rings in, `Glyph`s out,
  carrying contents, parent and links. **It names nothing**, and that is correct
  rather than unfinished: identifying ink is the recognizer's job and
  `templates.ron` is empty, so every seal compiles to rule 9's discharge. A test
  pins that, so the day it changes is a decision.
- **M5.8, the recorder.** `ui/record.rs` writes every non-ring stroke on the pad
  as a `templates.ron` fragment. Rings are excluded on purpose: a ring is the
  activator, not a rune, and the recogniser is never asked to identify one —
  `circle.rs` does that with geometry. The id lands as `RENAME_ME`, because
  naming the rune is a decision about canon and the panel has no text entry;
  making the file is mechanical and that is the half worth automating.
- Writing files is the shell's business (§4.1), and the format is core's —
  `templates::parse` has to accept the result, so the round trip is the test
  that matters.

*Reaches it from the panel (§4.8):* **`record`** writes the fragment and says on
the hint line what it wrote or why it wrote nothing. The **spell readout** now
compiles the whole pad through `compile_all`, so a nested or linked ring reports
what the other rings did to it — a ring compiled alone can never know either.

**Five bugs off the screenshot**, found by reading the overlay rather than the
code — which is what it is for:

- **`power -0.0`, `embed -0.00`, and a bare ring "leaning toward -180°".** IEEE's
  additive identity is *negative* zero, so an empty sum comes back `-0.0`, and
  `atan2(-0.0, -0.0)` is `-π`. A seal with no signs was reporting a firm
  direction. `+ 0.0` collapses the sign; `clamp` does not, which is why `embed`
  needed the same fix separately. `balance` now also refuses to report a heading
  when there is no drift to point — the field's own docs already said it was
  meaningless there, and the overlay printed it anyway.
- **"bare ring — this is an explosion", over a seal covered in ink.** Rule 9 is
  about a ring with *nothing* in it. A ring full of marks nobody can read is a
  spell we failed to read, and calling it an explosion is a lie with canon's name
  on it. `Glyph::unnamed` counts them and `Warning::Unreadable` says so. Until
  `templates.ron` holds traced runes that is every mark on the pad, and the count
  drops by one for each rune the recognizer learns.
- **`Radial` for a seal with no signs.** Vacuously true, and it reads as a
  measurement. The overlay prints `—` at zero signs; core still answers what it
  answers (§4.2).
- **Ring captions landing on the inspector's own lines.** Closed rings put their
  caption at the top, which is exactly where the readout runs. They sit at the
  bottom now.
- **The glaive was a fan, not a claw.** `spread.signum().max(0.4)` turned the
  left talon's `-1` into `+0.4`, curling all three the same way. Each talon now
  curls further out the way it already leans.

**The chevron bug** — found on screen after `sign` shipped, fixed before M5.2:

A placed sign's arrowhead was being reported as its own ring: `rings 4 from 10
strokes [r140 q0.95] [r11 q0.77] [r6 q0.77] [r7 q0.77]`. Two short lines meeting
at a point clear `min_span` outright — the fit puts the centre near the bend, so
the arms then sweep most of a turn about a radius of a few pixels — and they
clear `is_simple` too, because turning and coverage agree for a V just as they
do for an arc.

Nothing in the search asked **whether the ink was round at all**. That question
had been conflated with `min_quality`, which asks something else entirely and
only runs *after* a candidate is already a ring. Split into
`RingSearch::min_roundness` (0.85 — is this a circle) against
`RingRules::min_quality` (0.95 — is this ring neat enough to hold), so rule 8's
rough seal still seeds a ring and still grades `Fleeting`.

Rejecting on **size** would have been the wrong fix and is worth recording as
such: rule 5 links several small identical seals, and those are real rings.

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


---

## 11. Assets, and what is not one

`apps/canvas/assets/fonts/` holds **EB Garamond** and its licence, and that is
the only binary the repository carries.

**The typeface.** Bevy ships one font, a monospace, and it has two problems: no
box-drawing, no typographic dashes, no degree sign — which is why every string
this app prints had to be flattened to ASCII — and it looks like a terminal,
which is the opposite of what a book of spells looks like. EB Garamond is cut
after Claude Garamond's sixteenth-century romans, so it is the right century, and
it is under the **SIL Open Font License 1.1**, which permits embedding and
redistribution. `assets/fonts/OFL.txt` must stay beside it; that is the
condition.

Two hands, and picking between them is a decision each time. `Hand::script` for
anything a person *reads* — buttons, hints, the caption over a seal. Bevy's own
mono for anything in *columns*: a proportional serif makes the measurement
tables ragged, and a table you cannot scan is worse than an ugly one.

Baked in with `include_bytes!` rather than the asset server. `run.sh` builds a
real `.app` bundle, and an asset path that survives bundling is one more thing
to get wrong; M8's wasm build would need a third answer again. Eight hundred
kilobytes in the binary buys one answer that works everywhere.

**What the repository will not carry.** No manga panels, no anime frames, no
traced-and-recoloured versions of either. That is somebody's artwork, and a
hobby project with no income is still a copy — "non-commercial" is not a licence
and "it promotes the work" is not a defence. The rune shapes in
`templates.ron` are the same rule from the other direction: §2 has them traced
by a person from the source, not invented and not scraped.

None of that costs the project its look. The palette is a workbench — tooled
leather, iron-gall ink, candle-brass, verdigris — and every colour is one a dye
could have made. Colour is not copyrightable and a workshop is not a screenshot.


---

## 12. The built-in shapes

`shapes.rs` holds a geometry for fire, water, wind, earth and light, and for the
column, levitation and convergence signs.

**They are not traced from the manga.** §2 says the rune shapes belong to the
source and must be traced rather than invented, and `templates.ron` still ships
empty because nobody has done that. These are **reconstructions of the community
spell simulator's reconstructions** — simple geometry described in words and
rebuilt: fire is a triangle on a stem, water is three teardrops, wind is an S
with rays, earth is a bar over a chevron, light is a diamond in a square.

So they carry a fan-named sign's standing: good enough to build on, honest about
what they are, and **superseded the moment somebody traces the real thing**.
`templates::with_built_ins` appends them *after* whatever was recorded and skips
any name already taken, so a traced `fire` wins the day it exists.
`a_recorded_rune_outranks_the_built_in_of_the_same_name` pins that.

They live in **core** rather than in the shell because a shape is load-bearing
now: the recognizer matches against these, so what a fire sigil looks like
decides what a drawing *means*, and §4.2 keeps every decision about meaning in
core. The shell draws them; it does not define them.

Two tests do the work worth doing. `every_built_in_recognises_as_itself` — draw
one and the recognizer names it. `no_two_built_ins_are_the_same_drawing` — every
pair at least 0.15 apart, which is M4.8's own finding (a square and a ring at
0.110 and 0.127) turned into a standing check on the shapes we actually ship.
