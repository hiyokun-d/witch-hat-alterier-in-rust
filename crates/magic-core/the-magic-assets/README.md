# Canon Data Pack

Structured reference for the Witch Hat Atelier magic system, compiled from
the Independent Witch Hat Atelier Wiki (witchhatatelier.telepedia.net),
pages: Magic, Sigils Explained, Signs Explained, Spells, and individual
spell pages.

Drop this in `crates/magic-core/assets/canon/`.

## Files

| File | Purpose |
|---|---|
| `sigils.ron` | 13 sigils across 6 families, with capability flags |
| `signs.ron` | 30+ signs with class, tier, orientation semantics |
| `spells.ron` | Documented spell compositions — use as compiler test fixtures |

## Confidence tiers — respect these

Every entry carries a `confidence` field. This matters more than it looks:

- **`Canon`** — stated in the manga or established by the wiki. Safe to build
  mechanics on.
- **`Inferred`** — reconstructed by fans comparing spells that differ by one
  component. Usually right, occasionally wrong.
- **`Unknown`** — the name exists in the source, the effect does not. **Do not
  invent behaviour for these.** Model them as known-but-inert. Filling them in
  with guesses is how the system stops being learnable.

Signs also carry a `tier`: `Official` (named in source), `Unofficial`
(fan-named), `Decorative` (no mechanical effect).

## The three things that should change your engine design

**1. Capabilities, not just element types.**

Canon is precise about what each sigil can and cannot do:

| Sigil | create | manipulate | move |
|---|---|---|---|
| wind | ✗ | ✓ | ✓ |
| aeriforms | ✓ | ✓ | ✗ |
| earth | ✗ | ✓ | ✓ |
| water | ✓ (costly) | ✓ | ✓ |

Wind moves air but cannot make it. Aeriforms makes air but cannot move it.
Earth never creates matter at all. This hands you conservation laws for free —
most magic must *take* its material from the world, which is why `collection`
exists as a sign. Water can create, but canon notes long-duration water spells
collect instead, implying creation costs more. That's a cost model stated in
the lore.

**2. `pulling` gives you a continuous parameter.**

The most precisely specified sign in canon. Arrow inward at 0° is pure pull;
angled gives pull plus twist; rotated 90° is pure twist with no pull. That is
`pull = cos(θ)`, `twist = sin(θ)` — a real dial, not a boolean. Combined with
the `whorling_winds` sigil, vortices are fully canon and you never needed the
"rotate rune" we invented.

**3. An empty closed ring detonates.**

Closing a ring containing no sigils and no signs produces a large shockwave
and destroys the seal. Our earlier design returned `None` for this case, which
is wrong — it's an outcome, not an absence. Handle it as `Discharge`.

## Naming

Sigils end in 紋 (*mon*, "crest"). Signs end in 矢 (*ya*, "arrow"). Sigils are
nouns; signs are vectors. That distinction is worth carrying into the type
system: every `Sign` has an orientation, most `Sigil`s don't.

Known Japanese names: fire 炎の紋, light 光の紋, earth 地の紋, column 柱の矢,
pulling 引き寄せの矢, crushing 破砕の矢.

## Sign classes

The wiki groups signs into directional, semi-directional, non-directional,
and asymmetric. **These groups are the wiki's own analysis and are never named
in the manga.** They're mechanically useful, so they're preserved here — but
flag them as such if you ever surface them to a player. All directional signs
have bilateral symmetry and lack radial symmetry.

The wiki reports 44 signs deciphered so far; this pack covers the documented
subset.

## About the images

The sigil and sign artwork on the wiki is cropped from the manga — copyright
Kamome Shirahama / Kodansha. It isn't packaged here, for two reasons.

The practical one: **bitmaps are the wrong asset type for this engine.** The
$P recognizer matches point clouds — ordered stroke coordinates. A PNG of a
fire sigil is useless to it. Every glyph has to be drawn stroke-by-stroke to
become a template, so the images would only ever serve as visual reference
while drawing, which the wiki already does better in a browser.

The other one: this project is going on GitHub and hiyokun.dev. Shipping a
zip of manga panels in the repo is a liability that a portfolio piece really
doesn't need.

The path that works: build the M4.8 template recorder, open the wiki on a
second screen, and draw each glyph yourself. The output is `runes.ron` full of
your own stroke data — legally clean, and the only format the recognizer can
actually use. Original designs *inspired by* the system are safer still, and
canon explicitly supports invented signs (Richeh invented `weave` in-story).

## Sources

- Magic — https://witchhatatelier.telepedia.net/wiki/Magic
- Sigils Explained — https://witchhatatelier.telepedia.net/wiki/Sigils_Explained
- Signs Explained — https://witchhatatelier.telepedia.net/wiki/Signs_Explained
- Spells — https://witchhatatelier.telepedia.net/wiki/Spells

Wiki content is CC-BY-SA. This pack is a restructuring of factual game-system
information into a data format, written in our own words. Credit the wiki in
the project README.
