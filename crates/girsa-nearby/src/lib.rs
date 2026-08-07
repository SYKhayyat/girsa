//! What else is near this, and what the answer could not see (spec.md §9.9,
//! W30).
//!
//! Two modules, and the reason they are one crate above `girsa-app` rather than
//! two modules inside it is the manifest: [`girsa_lane`] is a BERT, three
//! `candle` crates and 738 MB of side-loaded weights, and the reading workspace
//! has no use for any of it. `cargo test -p girsa-app` used to build the
//! forward pass in order to retest the taxonomy.
//!
//! # Why `unseen` is here and not down in the workspace
//!
//! [`Unseen`] composes *what this answer could not see* for four surfaces — a
//! search, a lane answer, an MCP tool result, a printed line — and three of
//! those have nothing to do with the lane. It still lives here, because the one
//! sentence it has to be able to say is **"the lane has covered 240 of 7,189
//! seforim"**, and that number is a [`girsa_lane::Coverage`]. A composer that
//! could not name the lane's coverage would be a fourth composer, which is the
//! thing [`Unseen`] exists to stop being.
//!
//! So the dependency runs one way and the honesty is in one place: every
//! surface asks this crate, and this crate is the only one that has to know
//! what the lane knows.

pub mod adjacent;
pub mod unseen;

pub use adjacent::{Adjacency, Near};
pub use unseen::Unseen;
