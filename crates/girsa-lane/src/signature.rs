//! A vector's shape in 32 bytes, so a query can skip the other 3,040.
//!
//! # The hole this fills
//!
//! The 9 August report's §6 lists fourteen expensive things and calls thirteen
//! of them missed sweeps — a fix sitting twenty lines from the code that ignores
//! it. The fourteenth is this one, and it is named differently:
//!
//! > One genuine hole rather than a missed sweep: `girsa-lane`'s retrieval has
//! > no index. `Vectors::open` reads every record's id to build an offset map
//! > and `nearest` reads the whole file again — two full linear passes per sefer
//! > per query. At the offered scale (`Chosen::everything()`) that is ~15 GB read
//! > twice per query, and unlike the 13-day embed cost it is stated nowhere.
//!
//! Both halves are true and they have different answers. The repeated *open* is
//! a cache ([`crate::Lane`] holds its stores now, keyed on the file's length so
//! a store that grew is reopened). The repeated *read of every vector* is this
//! module: there is nothing to consult instead of the file, so something has to
//! be small enough to consult.
//!
//! # Why signed random projection
//!
//! Both sides are unit length, so nearness is the cosine of the angle between
//! them. Take a random hyperplane through the origin and ask which side of it a
//! vector falls on: two vectors an angle θ apart disagree with probability
//! θ/π. Ask 256 independent hyperplanes and the fraction of bits that disagree
//! *is* θ/π, to within the noise of 256 samples — so **Hamming distance between
//! two signatures estimates the angle between two vectors**, and a `u64` XOR
//! and four `count_ones` do what 768 multiplies did.
//!
//! That is 32 bytes where the vector is 3,072. Fifteen gigabytes becomes a
//! hundred and sixty megabytes — and the honest comparison is not against the
//! vectors, which were never in memory, but against what [`crate::Vectors`]
//! *already* holds: a `HashMap<SegmentId, u64>` and a `Vec<SegmentId>` over
//! every embedded segment, where a `SegmentId` is a good deal more than 32
//! bytes of heap. **The index is smaller than the id map it sits beside.**
//!
//! # The planes are not stored
//!
//! They are generated from a fixed seed by splitmix64, so every machine and
//! every rebuild produces the same 256 planes for a given width. Storing them
//! would be a second thing that can disagree with the signatures built from
//! them, for 786 KB of file and no benefit — nobody is ever going to want
//! *different* planes, and if that day comes it is a version bump, which the
//! sidecar header already carries.
//!
//! # What it is allowed to get wrong
//!
//! A signature is an estimate, so a shortlist drawn by Hamming distance can miss
//! a vector that the exact dot product would have ranked. [`crate::Vectors`]
//! answers that in three ways, and none of them is *hope*: the shortlist is
//! oversampled far past what is asked for, every vector on it is then scored
//! **exactly** by reading its record, and a store small enough to read whole is
//! read whole. What survives is reported — see `Vectors::nearest_reporting` —
//! rather than presented as if the file had been scanned.

/// How many hyperplanes, and therefore how many bits.
///
/// 256 puts the standard error of the angle estimate at about 3°, which is far
/// inside the gap between a shortlist and an answer — the shortlist is
/// oversampled by [`crate::vectors::OVERSAMPLE`] and then scored exactly, so a
/// bit of blur costs a candidate its place in a list nobody sees.
pub const BITS: usize = 256;

/// `BITS` as `u64`s.
pub const WORDS: usize = BITS / 64;

/// How many bytes one signature is on disk.
pub const BYTES: usize = BITS / 8;

/// One vector's side of each of the [`BITS`] planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Signature(pub [u64; WORDS]);

impl Signature {
    /// How many bits differ — an estimate of the angle, in units of π/[`BITS`].
    ///
    /// Smaller is nearer. Zero means the two vectors fell on the same side of
    /// all 256 planes, which is not the same as being equal and is not claimed
    /// to be: this is what puts a vector on a shortlist, never what ranks it.
    #[must_use]
    pub fn apart(self, other: Self) -> u32 {
        let mut out = 0;
        for i in 0..WORDS {
            out += (self.0[i] ^ other.0[i]).count_ones();
        }
        out
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; BYTES] {
        let mut out = [0u8; BYTES];
        for (i, word) in self.0.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    #[must_use]
    pub fn from_bytes(bytes: &[u8; BYTES]) -> Self {
        let mut words = [0u64; WORDS];
        for (i, word) in words.iter_mut().enumerate() {
            let mut eight = [0u8; 8];
            eight.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            *word = u64::from_le_bytes(eight);
        }
        Self(words)
    }
}

/// The seed the planes are grown from. Changing it invalidates every sidecar,
/// which is why [`crate::vectors::SIGNATURES`] carries a version byte.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// splitmix64 — a whole PRNG in four lines, with no dependency and no state
/// beyond a `u64`.
///
/// It is used because the planes must be identical on every machine and after
/// every rebuild. A seeded `rand` would do as well and would put a crate and a
/// version between this file and that guarantee.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The 256 hyperplanes for one vector width.
///
/// Held behind [`projection`], because building one is 786 KB and 200,000
/// draws, and a query over the whole shelf opens four thousand stores that all
/// want the same planes.
#[derive(Debug)]
pub struct Projection {
    dims: usize,
    /// `BITS × dims`, flat.
    planes: Vec<f32>,
}

impl Projection {
    /// Grow the planes for vectors of this width.
    #[must_use]
    pub fn for_dims(dims: usize) -> Self {
        let mut state = SEED ^ (dims as u64).wrapping_mul(0x2545_F491_4F6C_DD1D);
        let mut planes = Vec::with_capacity(BITS * dims);
        // Box–Muller, two normals per turn. Gaussian and not uniform because
        // the argument above — *two vectors θ apart disagree with probability
        // θ/π* — holds for a plane whose normal is drawn from a spherically
        // symmetric distribution, and a per-coordinate uniform draws from a
        // cube, which points at its own corners.
        let mut spare: Option<f32> = None;
        for _ in 0..BITS * dims {
            if let Some(held) = spare.take() {
                planes.push(held);
                continue;
            }
            // In (0, 1]: splitmix64 can return zero and `ln(0)` is not a number.
            let u1 = ((splitmix(&mut state) >> 11) as f64 + 1.0) / (1u64 << 53) as f64;
            let u2 = (splitmix(&mut state) >> 11) as f64 / (1u64 << 53) as f64;
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = std::f64::consts::TAU * u2;
            planes.push((r * theta.cos()) as f32);
            spare = Some((r * theta.sin()) as f32);
        }
        Self { dims, planes }
    }

    #[must_use]
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Which side of each plane this vector falls on.
    ///
    /// A vector of the wrong width signs as zero rather than panicking; the
    /// caller that could produce one is the one that already refuses it
    /// ([`crate::vectors::VectorError::WrongWidth`]), and a signature is a
    /// shortlist key, never an answer.
    #[must_use]
    pub fn sign(&self, vector: &[f32]) -> Signature {
        if vector.len() != self.dims {
            return Signature::default();
        }
        let mut words = [0u64; WORDS];
        for bit in 0..BITS {
            let plane = &self.planes[bit * self.dims..(bit + 1) * self.dims];
            let dot: f32 = plane.iter().zip(vector).map(|(p, v)| p * v).sum();
            if dot >= 0.0 {
                words[bit / 64] |= 1u64 << (bit % 64);
            }
        }
        Signature(words)
    }
}

/// The planes for a width, built once for the life of the process.
///
/// Four thousand stores opened by one query want the same 786 KB of planes, and
/// a query over the shelf opens four thousand stores. In practice this map has
/// one entry, because a lane has one model.
#[must_use]
pub fn projection(dims: usize) -> std::sync::Arc<Projection> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    static GROWN: OnceLock<Mutex<HashMap<usize, Arc<Projection>>>> = OnceLock::new();
    let grown = GROWN.get_or_init(|| Mutex::new(HashMap::new()));
    let mut grown = match grown.lock() {
        Ok(held) => held,
        // A panic while holding this lock would have been inside `for_dims`,
        // which does arithmetic and nothing else. Growing the planes again is
        // cheaper than propagating a poisoning nobody can act on.
        Err(poisoned) => poisoned.into_inner(),
    };
    Arc::clone(
        grown
            .entry(dims)
            .or_insert_with(|| Arc::new(Projection::for_dims(dims))),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn unit(values: &[f32]) -> Vec<f32> {
        let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        values.iter().map(|v| v / norm).collect()
    }

    #[test]
    fn the_planes_are_the_same_planes_every_time() {
        // The whole sidecar rests on this: signatures written by one run are
        // compared against a query signed by the next.
        let a = Projection::for_dims(64);
        let b = Projection::for_dims(64);
        let v = unit(&(0..64).map(|n| (n as f32) - 31.5).collect::<Vec<_>>());
        assert_eq!(a.sign(&v), b.sign(&v));
    }

    #[test]
    fn a_vector_is_no_distance_from_itself() {
        let p = Projection::for_dims(32);
        let v = unit(
            &(0..32)
                .map(|n| ((n * 7) % 13) as f32 - 6.0)
                .collect::<Vec<_>>(),
        );
        assert_eq!(p.sign(&v).apart(p.sign(&v)), 0);
    }

    #[test]
    fn hamming_distance_tracks_the_angle() {
        // The claim the whole index rests on: bits that differ ≈ θ/π × BITS.
        // Checked at three angles in a plane embedded in 128 dimensions, with a
        // tolerance of three standard errors — this is an estimator, and a test
        // that demanded the exact figure would be testing the seed.
        let p = Projection::for_dims(128);
        let mut a = vec![0.0f32; 128];
        a[0] = 1.0;
        for degrees in [15.0f64, 45.0, 90.0] {
            let mut b = vec![0.0f32; 128];
            let theta = degrees.to_radians();
            b[0] = theta.cos() as f32;
            b[1] = theta.sin() as f32;
            let bits = p.sign(&a).apart(p.sign(&b)) as f64;
            let want = theta / std::f64::consts::PI * BITS as f64;
            // Binomial standard error at p = θ/π over BITS trials.
            let q = theta / std::f64::consts::PI;
            let sigma = (q * (1.0 - q) * BITS as f64).sqrt();
            assert!(
                (bits - want).abs() <= 3.0 * sigma + 1.0,
                "{degrees}°: {bits} bits apart, expected about {want:.1} (σ {sigma:.1})"
            );
        }
    }

    #[test]
    fn a_signature_survives_the_disk() {
        let p = Projection::for_dims(48);
        let v = unit(
            &(0..48)
                .map(|n| ((n * 5) % 11) as f32 - 5.0)
                .collect::<Vec<_>>(),
        );
        let sig = p.sign(&v);
        assert_eq!(Signature::from_bytes(&sig.to_bytes()), sig);
    }

    #[test]
    fn the_planes_are_shared_rather_than_regrown() {
        // Not a performance assertion dressed as a test: four thousand stores
        // opened by one query each ask for these, and `Arc::ptr_eq` is the
        // difference between 786 KB and 3 GB.
        assert!(std::sync::Arc::ptr_eq(&projection(16), &projection(16)));
    }
}
