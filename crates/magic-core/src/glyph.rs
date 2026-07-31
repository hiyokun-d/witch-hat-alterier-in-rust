// this file will provide all of the signs and every types
// it's just a simple file, and i regret it okay

pub mod glyph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Elements {
    Fire,  // YEAHHHHH ON FIRE!!!
    Water, // this element bend water
    Earth, // this element is just like earth, i don't know how to describe
    Wind,  // this element will direct just a wind
    Light, // this element control how the light
}

pub use glyph::Elements;
