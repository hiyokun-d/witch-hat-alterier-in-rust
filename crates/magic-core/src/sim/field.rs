//! The space parcels live in: a fixed grid of cells, and the parcels themselves.
//!
//! A cell is a **summary**, never an authored thing. Density, temperature and
//! flow are all recomputed from the parcels by [`Field::settle`], and writing to
//! one directly would mean the parcel it should have come from is missing.
//!
//! §2.6 names temperature, velocity and density as the three fields elements
//! interact through, so those are the three a cell carries and there is not a
//! fourth.

use super::parcel::{Parcel, SubstanceId};
use super::vec2::Vec2;

/// One square of the grid — everything the parcels inside it add up to.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cell {
    /// Total mass sitting in this cell.
    pub density: f32,
    /// Mass-weighted mean temperature. Zero density means [`super::parcel::AMBIENT`].
    pub temperature: f32,
    /// Mass-weighted mean velocity.
    pub flow: Vec2,
    /// Mass-weighted mean *position* — where the mass in this cell actually
    /// sits, which is not the cell's centre.
    ///
    /// The piece pressure could not work without. A gradient sampled at cell
    /// centres cancels for any symmetric blob: a lone overdense cell surrounded
    /// by empty ones pushes equally in all four directions and therefore not at
    /// all. Measuring where the mass really is gives each parcel a direction to
    /// be pushed *away from*, which is what actually separates a pile.
    pub centroid: Vec2,
}

/// A grid of cells, and the parcels moving through it.
///
/// Every field is private: callers go through methods so `cells.len()` can never
/// drift from `width * height`, the same reason `Catalog` hides its maps.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    cells: Vec<Cell>,
    parcels: Vec<Parcel>,
    width: usize,
    height: usize,
    cell_size: f32,
    origin: Vec2,
}

impl Field {
    /// A grid `width x height` cells of `cell_size` pixels, with cell `(0, 0)`'s
    /// **corner** at `origin`.
    ///
    /// `Option` rather than a bare `Field`: a zero-width grid or a non-positive
    /// cell size is a caller's mistake, and §4.7 reports rather than panics on
    /// the divide that would follow.
    pub fn new(width: usize, height: usize, cell_size: f32, origin: Vec2) -> Option<Field> {
        // `is_finite` first, so a NaN cell size is refused rather than
        // sliding through a comparison that is false either way.
        if width == 0
            || height == 0
            || !cell_size.is_finite()
            || cell_size <= 0.0
            || !origin.is_finite()
        {
            return None;
        }
        Some(Field {
            cells: vec![Cell::default(); width * height],
            parcels: Vec::new(),
            width,
            height,
            cell_size,
            origin,
        })
    }

    /// A grid centred on `origin`, sized to cover `extent` pixels either way.
    ///
    /// The shape a shell actually wants: the pad is centred on the window, and
    /// working out a corner from that is arithmetic nobody should repeat.
    pub fn centered(cells_across: usize, cell_size: f32, center: Vec2) -> Option<Field> {
        let half = cells_across as f32 * cell_size * 0.5;
        Field::new(
            cells_across,
            cells_across,
            cell_size,
            center - Vec2::new(half, half),
        )
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    /// The bottom-left and top-right corners of the whole grid.
    pub fn bounds(&self) -> (Vec2, Vec2) {
        (
            self.origin,
            self.origin
                + Vec2::new(
                    self.width as f32 * self.cell_size,
                    self.height as f32 * self.cell_size,
                ),
        )
    }

    /// Row-major slot for a cell. Private: a raw index means nothing outside.
    fn index(&self, col: usize, row: usize) -> usize {
        row * self.width + col
    }

    /// Which cell a world position falls in, or `None` outside the grid.
    ///
    /// The negative test happens **before** the cast to `usize`, because casting
    /// `-1.0` to `usize` saturates to `0` in Rust and would silently file
    /// something off the left edge into the first column.
    pub fn cell_at(&self, at: Vec2) -> Option<(usize, usize)> {
        let local = at - self.origin;
        if !local.is_finite() || local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let col = (local.x / self.cell_size).floor();
        let row = (local.y / self.cell_size).floor();
        if col >= self.width as f32 || row >= self.height as f32 {
            return None;
        }
        Some((col as usize, row as usize))
    }

    /// Whether a position is inside the grid at all.
    pub fn contains(&self, at: Vec2) -> bool {
        self.cell_at(at).is_some()
    }

    pub fn cell(&self, at: Vec2) -> Option<&Cell> {
        let (col, row) = self.cell_at(at)?;
        self.cells.get(self.index(col, row))
    }

    /// One cell by grid coordinates, for a caller drawing the whole grid.
    pub fn cell_by(&self, col: usize, row: usize) -> Option<&Cell> {
        if col >= self.width || row >= self.height {
            return None;
        }
        self.cells.get(self.index(col, row))
    }

    /// The centre of a cell, in world coordinates.
    pub fn cell_center(&self, col: usize, row: usize) -> Vec2 {
        self.origin
            + Vec2::new(
                (col as f32 + 0.5) * self.cell_size,
                (row as f32 + 0.5) * self.cell_size,
            )
    }

    pub fn add(&mut self, parcel: Parcel) {
        self.parcels.push(parcel);
    }

    pub fn parcels(&self) -> &[Parcel] {
        &self.parcels
    }

    pub fn parcels_mut(&mut self) -> &mut [Parcel] {
        &mut self.parcels
    }

    /// Drops every parcel that has expired or been emptied.
    pub fn sweep(&mut self) {
        self.parcels.retain(Parcel::alive);
    }

    pub fn clear(&mut self) {
        self.parcels.clear();
        self.cells
            .iter_mut()
            .for_each(|cell| *cell = Cell::default());
    }

    /// Total mass across every parcel. The number M6.8 is written against.
    pub fn mass(&self) -> f32 {
        // `+ 0.0` because IEEE's additive identity is *negative* zero, and an
        // empty field reporting `-0.00` is the bug this project already shipped
        // once in `balance`.
        self.parcels.iter().map(|p| p.mass).sum::<f32>() + 0.0
    }

    /// Total mass of one substance.
    pub fn mass_of(&self, substance: &SubstanceId) -> f32 {
        self.parcels
            .iter()
            .filter(|p| &p.substance == substance)
            .map(|p| p.mass)
            .sum::<f32>()
            + 0.0
    }

    /// Total momentum, for the conservation tests.
    pub fn momentum(&self) -> Vec2 {
        self.parcels
            .iter()
            .fold(Vec2::ZERO, |sum, p| sum + p.momentum())
    }

    /// Total heat, in mass-degrees.
    pub fn heat(&self) -> f32 {
        self.parcels.iter().map(Parcel::heat).sum::<f32>() + 0.0
    }

    /// Takes up to `wanted` mass of `substance` from parcels within `radius` of
    /// `near`, and reports how much it actually got.
    ///
    /// **The conservation rule made real.** §3.2: wind moves air but cannot
    /// create it, earth manipulates stone but never makes it. A spell whose
    /// sigil cannot create has to find its substance, and in a room with no air
    /// it finds nothing — which is the whole point of encoding capabilities.
    ///
    /// Nearest first, so a spell drains what is under it before what is across
    /// the room. Ties are broken by position in the list, which is fixed, so the
    /// answer does not depend on iteration order (§4.3).
    pub fn take(&mut self, substance: &SubstanceId, wanted: f32, near: Vec2, radius: f32) -> f32 {
        if !wanted.is_finite() || wanted <= 0.0 {
            return 0.0;
        }

        let mut candidates: Vec<(usize, f32)> = self
            .parcels
            .iter()
            .enumerate()
            .filter(|(_, p)| &p.substance == substance && p.mass > 0.0)
            .map(|(i, p)| (i, (p.at - near).length()))
            .filter(|(_, distance)| *distance <= radius)
            .collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

        let mut taken = 0.0;
        for (index, _) in candidates {
            if taken >= wanted {
                break;
            }
            let parcel = &mut self.parcels[index];
            let bite = parcel.mass.min(wanted - taken);
            parcel.mass -= bite;
            taken += bite;
        }

        self.sweep();
        taken
    }

    /// Keeps every parcel inside the grid.
    ///
    /// **Ours** (§2.6), and the alternative was worse: a parcel that leaves has
    /// to either vanish or stop, and vanishing loses mass silently, which would
    /// make every conservation property in M6.8 untestable. So the grid has
    /// walls.
    ///
    /// `restitution` is how much speed survives the bounce, `0..=1`. It must
    /// not be `1.0`: a perfect wall is a trampoline, and the first version made
    /// a falling stone bounce forever — it never landed, so it never settled,
    /// and `a_stone_lands_and_stays_put` said so.
    pub fn confine(&mut self, restitution: f32) {
        let keep = restitution.clamp(0.0, 1.0);
        let (min, max) = self.bounds();
        // A hair inside, so a parcel resting exactly on the far edge still has
        // a cell — `cell_at` is half-open and `max` itself is outside.
        let skin = self.cell_size * 0.001;
        for parcel in &mut self.parcels {
            if parcel.at.x < min.x {
                parcel.at.x = min.x;
                parcel.velocity.x = parcel.velocity.x.abs() * keep;
            } else if parcel.at.x > max.x - skin {
                parcel.at.x = max.x - skin;
                parcel.velocity.x = -parcel.velocity.x.abs() * keep;
            }
            if parcel.at.y < min.y {
                parcel.at.y = min.y;
                parcel.velocity.y = parcel.velocity.y.abs() * keep;
            } else if parcel.at.y > max.y - skin {
                parcel.at.y = max.y - skin;
                parcel.velocity.y = -parcel.velocity.y.abs() * keep;
            }
        }
    }

    /// Merges parcels of the same substance sharing a cell into one.
    ///
    /// **The fix for a world that grows without bound.** Every reaction emits
    /// *new* product parcels, so a field where anything is reacting gains
    /// parcels every tick and never loses any: a room left running reached
    /// 36,000 parcels holding 400 units of mass — an average parcel weighing
    /// a hundredth of a unit, which is not a lump of anything.
    ///
    /// It also killed the numbers. A weighted mean over tens of thousands of
    /// near-zero masses is where `momentum NaN, NaN` came from, and a NaN in a
    /// velocity spreads to everything it touches on the next tick.
    ///
    /// Exact, not approximate: mass adds, momentum adds, heat adds, and the
    /// merged parcel's velocity and temperature are the mass-weighted means. A
    /// parcel with an anchor or a shorter life is left alone — those are facts
    /// about one parcel and merging would silently discard them.
    pub fn coalesce(&mut self) {
        let width = self.width;
        // Not down to *one* per cell. Pressure separates parcels by pushing
        // them away from where their cell's mass actually sits, and a single
        // parcel is always exactly there — it has no direction to be pushed in,
        // so merging all the way collapsed every pool into an immovable dot.
        // A handful per cell is still a bounded count and still has a gradient.
        const KEEP_PER_CELL: usize = 4;
        // Rebuilt rather than filtered in place. The first version walked
        // indices and kept a `merged` flag per parcel, which meant indexing two
        // collections by the same counter — correct, and exactly the shape
        // clippy asks you to justify. Moving the parcels out and pushing the
        // survivors says the same thing without an index anywhere.
        let taken = std::mem::take(&mut self.parcels);
        let mut keep: Vec<Parcel> = Vec::with_capacity(taken.len());
        // (cell index, substance, slot in `keep`) for whatever may still absorb.
        let mut open: Vec<(usize, usize)> = Vec::new();

        for parcel in taken {
            // A parcel with an anchor or a countdown is not interchangeable
            // with its neighbours: both are facts about that one parcel, and
            // merging would silently discard them.
            let mergeable = parcel.anchor.is_none() && parcel.life.is_infinite();
            let cell = self.cell_at(parcel.at).map(|(col, row)| row * width + col);

            let host = match (mergeable, cell) {
                (true, Some(slot)) => {
                    let peers: Vec<usize> = open
                        .iter()
                        .filter(|(at, into)| {
                            *at == slot && keep[*into].substance == parcel.substance
                        })
                        .map(|(_, into)| *into)
                        .collect();
                    // Merge into the lightest, so a cell's parcels stay of
                    // comparable size rather than one hoovering up the rest.
                    if peers.len() < KEEP_PER_CELL {
                        None
                    } else {
                        peers
                            .into_iter()
                            .min_by(|a, b| keep[*a].mass.total_cmp(&keep[*b].mass))
                    }
                }
                _ => None,
            };

            match host {
                Some(into) => {
                    let host = &mut keep[into];
                    let total = host.mass + parcel.mass;
                    if total > 0.0 {
                        host.velocity = (host.velocity * host.mass + parcel.velocity * parcel.mass)
                            * (1.0 / total);
                        host.temperature = (host.temperature * host.mass
                            + parcel.temperature * parcel.mass)
                            / total;
                        host.at = (host.at * host.mass + parcel.at * parcel.mass) * (1.0 / total);
                        host.mass = total;
                    }
                }
                None => {
                    if let (true, Some(slot)) = (mergeable, cell) {
                        open.push((slot, keep.len()));
                    }
                    keep.push(parcel);
                }
            }
        }

        self.parcels = keep;
    }

    /// Replaces any parcel the maths has ruined.
    ///
    /// A `NaN` velocity is not a value, it is a bug that has already happened —
    /// and unlike a panic it spreads silently, because every sum it enters
    /// comes out `NaN` too. §4.7 says no panics, so the only honest thing left
    /// is to notice and stop it. Returns how many it caught, so a readout can
    /// say so rather than the field quietly healing itself.
    pub fn scrub(&mut self) -> usize {
        let mut caught = 0;
        for parcel in &mut self.parcels {
            if !parcel.at.is_finite() {
                parcel.at = self.origin;
                caught += 1;
            }
            if !parcel.velocity.is_finite() {
                parcel.velocity = Vec2::ZERO;
                caught += 1;
            }
            if !parcel.temperature.is_finite() || !parcel.mass.is_finite() {
                parcel.temperature = super::parcel::AMBIENT;
                parcel.mass = 0.0;
                caught += 1;
            }
        }
        if caught > 0 {
            self.parcels.retain(Parcel::alive);
        }
        caught
    }

    /// Drops every parcel that has left the grid.
    ///
    /// The alternative to walls. Mass falls when something escapes, and that is
    /// the point: it is visible in the readout instead of being quietly kept.
    pub fn spill(&mut self) {
        let (min, max) = self.bounds();
        self.parcels.retain(|parcel| {
            parcel.at.x >= min.x
                && parcel.at.x <= max.x
                && parcel.at.y >= min.y
                && parcel.at.y <= max.y
        });
    }

    /// Recomputes every cell from the parcels.
    ///
    /// Mass-weighted, so a speck cannot outvote a boulder, and idempotent — it
    /// runs every frame and calling it twice must not move anything.
    ///
    /// The sum walks `parcels` in slice order. That is safe **only** because
    /// that order is fixed; float addition is not associative, and summing the
    /// same set in two orders is exactly the determinism bug M5.6 found in
    /// `balance`. Anything that sorts or filters this list changes the last
    /// decimal.
    pub fn settle(&mut self) {
        let (width, cell_size, origin) = (self.width, self.cell_size, self.origin);
        for cell in &mut self.cells {
            *cell = Cell::default();
        }

        for parcel in &self.parcels {
            let Some((col, row)) = self.cell_at(parcel.at) else {
                // Outside the grid contributes to nothing. `confine` is what
                // stops this happening; a parcel placed outside by a caller is
                // simply not in the world yet.
                continue;
            };
            let slot = row * self.width + col;
            let cell = &mut self.cells[slot];
            cell.density += parcel.mass;
            cell.temperature += parcel.heat();
            cell.flow += parcel.velocity * parcel.mass;
            cell.centroid += parcel.at * parcel.mass;
        }

        for (slot, cell) in self.cells.iter_mut().enumerate() {
            if cell.density > 0.0 {
                cell.temperature /= cell.density;
                cell.flow = cell.flow * (1.0 / cell.density);
                cell.centroid = cell.centroid * (1.0 / cell.density);
            } else {
                cell.temperature = super::parcel::AMBIENT;
                cell.flow = Vec2::ZERO;
                // An empty cell's centroid is its own centre: nothing is there,
                // so the only honest answer is the middle of the square.
                let (col, row) = (slot % width, slot / width);
                cell.centroid = origin
                    + Vec2::new(
                        (col as f32 + 0.5) * cell_size,
                        (row as f32 + 0.5) * cell_size,
                    );
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/field.rs"]
mod tests;
