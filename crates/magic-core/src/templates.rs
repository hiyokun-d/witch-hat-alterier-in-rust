//! Recorded gestures, loaded from data.
//!
//! A template is a drawing of a sigil or a sign that ink can be matched
//! against. There is no `match` arm anywhere that says what a fire sigil looks
//! like, and there must not be (§4.4): adding a rune has to be a change to a
//! `.ron` file, not a rebuild.
//!
//! Nothing here touches the filesystem. Callers hand in the file's contents as
//! a string, exactly as [`Catalog`] does, so core still compiles to wasm
//! untouched (§4.1).
//!
//! # Recording a template is tracing, not inventing
//!
//! The shapes belong to the manga. This module can load them, check their
//! names against the catalogue, and match ink to them; it cannot know what
//! they look like. `the-magic-assets/templates.ron` therefore ships with the
//! format documented and no entries — an empty list is an honest statement
//! that the tracing has not happened yet, and a made-up fire sigil would not
//! be.

use serde::Deserialize;

use crate::Point;
use crate::catalog::{Catalog, CatalogError, SigilId, SignId};
use crate::recognizer::{self, Template};

/// Which vocabulary a template's id belongs to.
///
/// Kept because the two are separate namespaces: nothing stops a sigil and a
/// sign sharing a name, and the catalogue would answer differently for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Kind {
    Sigil,
    Sign,
}

/// A gesture as it sits in the file: an id, and the strokes it was drawn with.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RecordedGesture {
    id: String,
    kind: Kind,
    /// One list of `(x, y)` per stroke. Several strokes are one gesture — most
    /// runes take more than one mark.
    strokes: Vec<Vec<(f32, f32)>>,
}

impl RecordedGesture {
    /// Flattens the strokes into points, numbering each stroke in turn.
    ///
    /// The numbering is positional rather than recorded, because it carries no
    /// meaning beyond "these points were one mark" — and stroke order is never
    /// an input to anything downstream (§3.3).
    fn points(&self) -> Vec<Point> {
        self.strokes
            .iter()
            .enumerate()
            .flat_map(|(stroke_id, stroke)| {
                stroke.iter().map(move |&(x, y)| Point {
                    x,
                    y,
                    stroke_id: stroke_id as u32,
                })
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct TemplateFile {
    /// Bumped when the shape of this file changes. A recorded gesture is
    /// expensive to produce, so a silently misread file would be costly.
    version: u32,
    templates: Vec<RecordedGesture>,
}

/// The file layout this module understands.
pub const FORMAT_VERSION: u32 = 1;

/// A template loaded from data, and which vocabulary it names.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub kind: Kind,
    pub template: Template,
}

/// Parses a template file, normalising each gesture to `n` points.
///
/// `file` only ever appears in error messages, so a caller can say which of
/// several files went wrong.
///
/// Rejects a file whose version is not [`FORMAT_VERSION`], a duplicate id
/// within one kind, and any gesture that cannot be normalised — a rune with
/// one point, or every point in one place, is a recording mistake and silently
/// dropping it would leave a rune that never matches and never explains why.
pub fn parse(file: &'static str, source: &str, n: usize) -> Result<Vec<Recorded>, CatalogError> {
    let parsed: TemplateFile =
        ron::Options::default()
            .from_str(source)
            .map_err(|error| CatalogError::Parse {
                file,
                message: error.to_string(),
            })?;

    if parsed.version != FORMAT_VERSION {
        return Err(CatalogError::Parse {
            file,
            message: format!(
                "version {} but this build reads version {FORMAT_VERSION}",
                parsed.version
            ),
        });
    }

    let mut out = Vec::with_capacity(parsed.templates.len());
    for gesture in &parsed.templates {
        // Several samples of one rune is **wanted**, not an error.
        //
        // This used to be rejected, on the reasoning that "one of them can
        // never win a match, and which one is silently decided by file order".
        // That is wrong for a nearest-neighbour recognizer: both compete, the
        // nearer wins, and since they carry the same name the *answer* is the
        // same either way. What varies is which sample of your handwriting the
        // drawing happens to be closest to — which is the entire reason to
        // record more than one.
        //
        // Three to five samples of a hand-drawn shape is the usual figure, and
        // it is the cheapest accuracy there is.

        let Some(template) = Template::record(gesture.id.clone(), &gesture.points(), n) else {
            return Err(CatalogError::Parse {
                file,
                message: format!(
                    "{:?} has no shape to record — too few points, or all in one place",
                    gesture.id
                ),
            });
        };

        out.push(Recorded {
            kind: gesture.kind,
            template,
        });
    }

    Ok(out)
}

/// Checks every template names something the catalogue defines.
///
/// Separate from [`parse`] so that a set of shapes can be loaded and tested
/// without a catalogue — the recogniser does not care what a name means, and
/// the machinery should be testable without the whole vocabulary present.
pub fn check(recorded: &[Recorded], catalog: &Catalog) -> Result<(), CatalogError> {
    for entry in recorded {
        let known = match entry.kind {
            Kind::Sigil => catalog
                .sigil(&SigilId(entry.template.name.clone()))
                .is_some(),
            Kind::Sign => catalog.sign(&SignId(entry.template.name.clone())).is_some(),
        };

        if !known {
            return Err(CatalogError::UnknownReference {
                from: "templates.ron".to_string(),
                to: entry.template.name.clone(),
            });
        }
    }

    Ok(())
}

/// The templates alone, ready for [`recognizer::rank`] or
/// [`recognizer::classify`].
pub fn templates(recorded: &[Recorded]) -> Vec<Template> {
    recorded.iter().map(|r| r.template.clone()).collect()
}

/// The built-in shapes, as templates the recognizer can match against.
///
/// `templates.ron` still ships empty and still means what it meant: nobody has
/// traced the manga's own line art. These are the reconstructions in
/// [`crate::shapes`], and they exist because an empty catalogue makes the whole
/// recognizer unreachable — every seal compiles to rule 9's discharge and the
/// compiler may as well not be there.
///
/// **A traced rune of the same name must win.** [`with_built_ins`] appends
/// these *after* whatever was loaded, and `classify` keeps the earlier of two
/// equal distances, so a recorded `fire` outranks the built-in `fire` the day
/// somebody records one.
pub fn built_ins(n: usize) -> Vec<Recorded> {
    crate::shapes::built_in()
        .into_iter()
        .filter_map(|(id, kind, strokes)| {
            let points: Vec<Point> = strokes
                .iter()
                .enumerate()
                .flat_map(|(stroke_id, stroke)| {
                    stroke.iter().map(move |&(x, y)| Point {
                        x,
                        y,
                        stroke_id: stroke_id as u32,
                    })
                })
                .collect();
            Some(Recorded {
                kind,
                template: Template::record(id, &points, n)?,
            })
        })
        .collect()
}

/// Whatever was recorded, then the built-ins as a fallback.
pub fn with_built_ins(recorded: Vec<Recorded>, n: usize) -> Vec<Recorded> {
    let mut all = recorded;
    for shape in built_ins(n) {
        // A recorded rune of the same name and kind is the better answer, so
        // the built-in stands aside rather than competing with it.
        let taken = all
            .iter()
            .any(|have| have.kind == shape.kind && have.template.name == shape.template.name);
        if !taken {
            all.push(shape);
        }
    }
    all
}

/// Which recorded gesture a drawing is nearest, by name.
///
/// A convenience over [`recognizer::classify`] for the common case of having a
/// loaded set to hand. Returns `None` only when nothing could be compared.
pub fn identify(points: &[Point], recorded: &[Recorded], n: usize) -> Option<(String, f32)> {
    let cloud = recognizer::normalize(points, n)?;
    let all = templates(recorded);
    let found = recognizer::classify(&cloud, &all)?;
    Some((all[found.index].name.clone(), found.distance))
}

#[cfg(test)]
#[path = "tests/templates.rs"]
mod tests;
