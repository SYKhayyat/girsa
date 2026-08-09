//! Where the vectors live, and why they cannot be quietly mixed.
//!
//! `personal/lane/<slug>/vectors.bin` — one file per sefer, under the personal
//! root beside the readings, the corrections and the link repairs, under the
//! rule all of those are under: **nothing here writes into `corpus/`.** An
//! embedding is a model's opinion about a text, and this project has been
//! careful from W20 onwards about not letting an opinion become the text.
//!
//! # The header is the point of the file format
//!
//! Every file records **which model made it**. Vectors from two checkpoints are
//! not comparable — the spaces are unrelated, the numbers are the same size and
//! the arithmetic runs happily — so mixing them yields a ranked list that looks
//! exactly like a good one and is noise. There is no way for a reader to notice
//! that from the results. So the file says what made it, and a model that did
//! not make it is refused at the door with both names
//! ([`Vectors::made_by_something_else`]), which the coverage line then says out
//! loud.
//!
//! That is spec.md §9's rule one layer down: *the engine never changes your
//! query without you knowing* is worth nothing if the index silently answers
//! from a different space than the query was embedded into.
//!
//! # Append, and a torn tail
//!
//! The same argument `girsa-scan`'s `words.jsonl` makes: **the work product is
//! the progress record.** The vectors in the file are the segments that are
//! done, so there is no second file that can survive a crash while disagreeing
//! with what was actually embedded, and the job resumes by reading it.
//!
//! Binary rather than JSONL because a 768-float vector is 3 KB as bytes and
//! about 10 KB as text, and the whole corpus is 5,000,545 segments. The cost of
//! that choice is that a crash mid-write leaves a **torn tail** rather than a
//! half-line, so a record is written with a tag byte in front of it and reading
//! stops at the first byte that is not one. What was ignored is **counted and
//! reported**, never silently dropped: a sefer that lost its last four vectors
//! must come back round the queue, and a sefer that lost them silently would be
//! reported as covered.

use std::collections::HashMap;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use girsa_corpus::import::slug_dir;
use girsa_corpus::segment::SegmentId;

/// What the file begins with, so that something which is not one of these is
/// refused rather than parsed.
const MAGIC: &[u8; 12] = b"GIRSA-LANE\x01\n";

/// In front of every record. A crash mid-append leaves a tail that does not
/// start with this, which is how a torn write is told from a short file.
const RECORD: u8 = 0x1e;

/// What the signature sidecar begins with.
///
/// The version byte covers the plane seed and the bit count as well as the
/// layout: change any of them and every signature in the file means something
/// else, and a sidecar that quietly meant something else would rank noise.
const SIGNATURES: &[u8; 12] = b"GIRSA-SIGS\x01\n";

/// How many candidates are shortlisted per answer wanted.
///
/// The shortlist is scored exactly afterwards, so this buys recall and costs
/// reads: at 32, asking for the best ten reads 320 records — about a megabyte of
/// an eight-megabyte file — and a candidate has to be beaten by 320 others on an
/// estimate with a three-degree standard error to be lost.
pub const OVERSAMPLE: usize = 32;

/// The shortlist never goes below this, however few were asked for. Asking for
/// one is the case an estimator is worst at.
const SHORTLIST_LEAST: usize = 256;

/// A store with fewer vectors than this is read whole.
///
/// Below it the index is not worth its seeks: the exact pass is one sequential
/// read of a few megabytes, and the shortlist would be most of the file anyway.
/// Above it the exact pass is the 15 GB the report is about.
const EXACT_UNDER: usize = 4_096;

/// Why a store would not open or would not write.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a lane store")]
    NotAStore { path: String },
    /// Refused rather than appended. The two vectors would be the same length
    /// and would rank against each other perfectly happily.
    #[error(
        "these vectors were made by {made_by} and the model configured now is {configured} — \
         re-embed this sefer, or point the lane back at the model that made them"
    )]
    OtherModel { made_by: String, configured: String },
    #[error("a {dims}-dimension store was given a vector of {given}")]
    WrongWidth { dims: usize, given: usize },
}

impl VectorError {
    fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// One sefer's vectors.
#[derive(Debug)]
pub struct Vectors {
    path: Option<PathBuf>,
    dims: usize,
    fingerprint: String,
    /// Where each id's newest record starts in the file. The id index and the
    /// order together; the vectors themselves stay on disk.
    ///
    /// Holding them in memory would be 15 GB over the whole corpus, and the
    /// whole corpus is a selection a reader is explicitly allowed to make. So a
    /// query reads the file — see [`Vectors::nearest`] — and what is kept here
    /// is only what the job needs to know it is done.
    at: HashMap<SegmentId, u64>,
    /// Ids in the order their winning record appears, so a ranking's ties break
    /// by reading order rather than by however a hash map felt.
    order: Vec<SegmentId>,
    /// Set when the file was made by another model. Nothing is read from it and
    /// nothing may be appended to it until it is [`Vectors::restarted`].
    made_by: Option<String>,
    /// Bytes at the end of the file that are not a record — a crash mid-write.
    torn: u64,
    /// Each record's 32-byte signature, by the offset the record starts at.
    ///
    /// The index the 9 August report says this store does not have. See
    /// [`crate::signature`] for what a signature is and why it is small enough
    /// to hold when the vectors are not; see [`Vectors::signatures`] for why the
    /// sidecar it comes from cannot silently disagree with `vectors.bin`.
    ///
    /// Empty is a valid state and means *rank the honest way*: a sidecar that
    /// would not build, a read-only personal layer, a store small enough not to
    /// bother. [`Vectors::nearest`] reads the whole file in that case and says
    /// it did.
    sig: HashMap<u64, crate::signature::Signature>,
}

/// A vector, and the segment it is of.
#[derive(Debug, Clone)]
pub struct Vector {
    pub id: SegmentId,
    pub vector: Vec<f32>,
}

/// A ranking, and how much of the file it looked at.
///
/// The second field is not diagnostics — it is the same shape
/// [`girsa_link::chain::Refused`] has and for the same reason. An answer drawn
/// from a shortlist and an answer drawn from the whole store look identical, so
/// the store says which it gave rather than leaving a reader to assume.
#[derive(Debug, Clone, Default)]
pub struct Ranked {
    /// Best first. Every score is an exact dot product.
    pub best: Vec<(SegmentId, f32)>,
    /// Every vector in the store was read and scored. When false the ranking is
    /// over a shortlist drawn by [`crate::signature`], and a vector the estimate
    /// misjudged is not in it.
    pub whole: bool,
    /// How many records were read off disk to produce this.
    pub read: usize,
}

impl Vectors {
    /// Where one sefer's vectors sit under a personal layer.
    #[must_use]
    pub fn dir_in(personal: &Path, slug: &str) -> PathBuf {
        slug_dir(&personal.join("lane"), slug)
    }

    #[must_use]
    pub fn path_in(personal: &Path, slug: &str) -> PathBuf {
        Self::dir_in(personal, slug).join("vectors.bin")
    }

    /// Open one sefer's vectors for the model named by `fingerprint`.
    ///
    /// Trouble is returned rather than raised: one unreadable sefer may not
    /// cost a reader the other four thousand, and it may not be silent either.
    ///
    /// A file made by another model opens **empty**, with
    /// [`Vectors::made_by_something_else`] set. Empty rather than deleted: the
    /// reader may have pointed the setting at the wrong directory, and throwing
    /// away a week of embedding to punish a typo is not a repair.
    #[must_use]
    pub fn open(
        personal: &Path,
        slug: &str,
        fingerprint: &str,
        dims: usize,
    ) -> (Self, Vec<String>) {
        let path = Self::path_in(personal, slug);
        let mut store = Self {
            path: Some(path.clone()),
            dims,
            fingerprint: fingerprint.to_string(),
            at: HashMap::new(),
            order: Vec::new(),
            made_by: None,
            torn: 0,
            sig: HashMap::new(),
        };
        let mut trouble = Vec::new();
        if let Err(e) = store.read(&path) {
            trouble.push(e.to_string());
        }
        // After `read`, because it needs the offsets `read` found and the
        // length it stopped at. Trouble here is never fatal: an index that will
        // not build costs a slow query, not an answer, and `nearest` says which
        // it gave.
        if let Err(e) = store.signatures(&path) {
            trouble.push(format!("{e} — ranking will read the whole file"));
        }
        if store.torn > 0 {
            trouble.push(format!(
                "{}: the last {} byte{} are not a whole record and were ignored — those segments \
                 will be embedded again",
                path.display(),
                store.torn,
                if store.torn == 1 { "" } else { "s" }
            ));
        }
        (store, trouble)
    }

    /// A store that is not backed by a file, for a caller with no personal
    /// layer and for a test.
    #[must_use]
    pub fn nowhere(fingerprint: &str, dims: usize) -> Self {
        Self {
            path: None,
            dims,
            fingerprint: fingerprint.to_string(),
            at: HashMap::new(),
            order: Vec::new(),
            made_by: None,
            torn: 0,
            sig: HashMap::new(),
        }
    }

    /// What made the vectors on disk, when it is not the model configured now.
    #[must_use]
    pub fn made_by_something_else(&self) -> Option<&str> {
        self.made_by.as_deref()
    }

    #[must_use]
    pub fn dims(&self) -> usize {
        self.dims
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Whether this segment has a vector under the model configured now.
    #[must_use]
    pub fn has(&self, id: &SegmentId) -> bool {
        self.at.contains_key(id)
    }

    /// Write one down.
    ///
    /// # Errors
    ///
    /// If the vector is the wrong width, if the file was made by another model,
    /// or if the personal layer will not take it.
    pub fn record(&mut self, id: &SegmentId, vector: &[f32]) -> Result<(), VectorError> {
        if vector.len() != self.dims {
            return Err(VectorError::WrongWidth {
                dims: self.dims,
                given: vector.len(),
            });
        }
        if let Some(made_by) = &self.made_by {
            return Err(VectorError::OtherModel {
                made_by: made_by.clone(),
                configured: self.fingerprint.clone(),
            });
        }
        let mut at = 0;
        if let Some(path) = self.path.clone() {
            at = self.append(&path, id, vector)?;
        }
        if self.at.insert(id.clone(), at).is_none() {
            self.order.push(id.clone());
        }
        Ok(())
    }

    /// Throw the file away and begin again under the model configured now.
    ///
    /// The one thing that clears [`Vectors::made_by_something_else`], and it is
    /// deliberately a separate call: it destroys work, so it is something a
    /// reader asks for rather than something an open does on their behalf.
    ///
    /// # Errors
    ///
    /// If the personal layer will not write.
    pub fn restarted(&mut self) -> Result<(), VectorError> {
        self.at.clear();
        self.order.clear();
        self.sig.clear();
        self.made_by = None;
        self.torn = 0;
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(VectorError::io(dir))?;
        }
        let mut file = std::fs::File::create(&path).map_err(VectorError::io(&path))?;
        file.write_all(&self.header())
            .map_err(VectorError::io(&path))?;
        Ok(())
    }

    /// The nearest segments to a query vector, best first.
    ///
    /// The vectors on both sides are unit length ([`crate::model`] normalizes
    /// on the way out), so this is a dot product and the number is a cosine
    /// between −1 and 1.
    ///
    /// The vectors on both sides are unit length, so this is a dot product and
    /// nothing else.
    ///
    /// # Errors
    ///
    /// If the file will not read, or was made by another model.
    pub fn nearest(
        &self,
        query: &[f32],
        most: usize,
    ) -> Result<Vec<(SegmentId, f32)>, VectorError> {
        Ok(self.nearest_reporting(query, most)?.best)
    }

    /// [`Vectors::nearest`], and how it got the answer.
    ///
    /// # The two ways, and why a caller is told which
    ///
    /// **Whole**, for a store under [`EXACT_UNDER`] vectors or one with no
    /// index: every record is read and scored, so the answer is the answer.
    ///
    /// **By the index**, otherwise: [`crate::signature`] estimates the angle
    /// from 32 bytes a record, the best [`OVERSAMPLE`]×`most` of those are read
    /// and scored **exactly**, and the ranking is over that shortlist. The
    /// scores are true scores — an estimate never reaches the answer — but a
    /// vector that the estimate put 400th and the exact score would have put
    /// 3rd is gone, and no amount of oversampling makes that impossible.
    ///
    /// So it is reported. Rule 6, one layer down: the crate that refuses to mix
    /// two models' vectors because *"there is no way for a reader to notice that
    /// from the results"* does not get to quietly answer from a shortlist
    /// either.
    ///
    /// # Errors
    ///
    /// If the file will not read, or was made by another model.
    pub fn nearest_reporting(&self, query: &[f32], most: usize) -> Result<Ranked, VectorError> {
        if let Some(made_by) = &self.made_by {
            return Err(VectorError::OtherModel {
                made_by: made_by.clone(),
                configured: self.fingerprint.clone(),
            });
        }
        if most == 0 || query.len() != self.dims || self.order.is_empty() {
            return Ok(Ranked::default());
        }
        // Kept sorted at `most` rather than sorted at the end. A lane over a
        // whole sefer is 18,120 vectors and this asks for ten of them, so the
        // full sort was ordering 18,110 rows nobody reads — once per sefer per
        // query.
        //
        // **Still stable**, which the truncated sort was and which matters: the
        // insertion point is the first row this one *beats*, so an equal score
        // stays behind the one already there — and the file's order is the
        // order they were embedded in, which is reading order.
        let mut best: Vec<(SegmentId, f32)> = Vec::with_capacity(most);
        let mut keep = |id: SegmentId, near: f32| {
            if best.len() == most && !best.last().is_some_and(|(_, worst)| near > *worst) {
                return;
            }
            let at = best.partition_point(|(_, held)| *held >= near);
            best.insert(at, (id, near));
            best.truncate(most);
        };

        if self.sig.len() < self.order.len() || self.order.len() < EXACT_UNDER {
            // Read whole. Either the store is small enough that the shortlist
            // would be most of it, or the index is not there for every record —
            // and a partial index is not a shortlist, it is a silent omission.
            let mut read = 0usize;
            for vector in self.all()? {
                let vector = vector?;
                read += 1;
                let near: f32 = query.iter().zip(&vector.vector).map(|(a, b)| a * b).sum();
                keep(vector.id, near);
            }
            return Ok(Ranked {
                best,
                whole: true,
                read,
            });
        }

        // Stage one, in memory: the whole store ranked by an estimate that is
        // an XOR and four `count_ones`.
        let want = (most * OVERSAMPLE).max(SHORTLIST_LEAST).min(self.sig.len());
        let asked = crate::signature::projection(self.dims).sign(query);
        let mut near: Vec<(u32, u64)> = self
            .sig
            .iter()
            .map(|(at, s)| (s.apart(asked), *at))
            .collect();
        // `select_nth_unstable` and not a sort: the shortlist's own order does
        // not survive stage two, which scores every one of them exactly.
        let cut = want.saturating_sub(1).min(near.len().saturating_sub(1));
        near.select_nth_unstable(cut);
        near.truncate(want);
        // In file order. These are seeks into an eight-megabyte file and a
        // shortlist read back-to-front would ask the disk to walk it backwards
        // for no reason.
        near.sort_unstable_by_key(|(_, at)| *at);

        // Stage two, on disk: the exact score, for the shortlist only.
        let mut read = 0usize;
        for (_, at) in near {
            if let Some(vector) = self.record_at(at)? {
                read += 1;
                let score: f32 = query.iter().zip(&vector.vector).map(|(a, b)| a * b).sum();
                keep(vector.id, score);
            }
        }
        Ok(Ranked {
            best,
            whole: false,
            read,
        })
    }

    /// Every vector in the file, in the order it was written.
    ///
    /// # Errors
    ///
    /// If the file will not open.
    pub fn all(
        &self,
    ) -> Result<impl Iterator<Item = Result<Vector, VectorError>> + '_, VectorError> {
        let reader = match &self.path {
            Some(path) if path.is_file() => {
                let file = std::fs::File::open(path).map_err(VectorError::io(path))?;
                let mut reader = BufReader::new(file);
                reader
                    .seek(SeekFrom::Start(self.header().len() as u64))
                    .map_err(VectorError::io(path))?;
                Some((reader, path.clone()))
            }
            _ => None,
        };
        let newest: HashMap<&SegmentId, u64> = self.at.iter().map(|(id, at)| (id, *at)).collect();
        Ok(Records {
            reader,
            dims: self.dims,
            at: self.header().len() as u64,
            newest,
        })
    }

    /// Where the signature sidecar sits, beside the vectors it is of.
    fn sidecar(path: &Path) -> PathBuf {
        path.with_file_name("signatures.bin")
    }

    /// The bytes the sidecar begins with, up to but not including `covers`.
    fn sidecar_header(&self) -> Vec<u8> {
        let mut header = SIGNATURES.to_vec();
        header.extend_from_slice(&u32::try_from(self.dims).unwrap_or(0).to_le_bytes());
        header.extend_from_slice(
            &u16::try_from(crate::signature::BITS)
                .unwrap_or(0)
                .to_le_bytes(),
        );
        let name = self.fingerprint.as_bytes();
        let len = u16::try_from(name.len()).unwrap_or(0);
        header.extend_from_slice(&len.to_le_bytes());
        header.extend_from_slice(&name[..len as usize]);
        header
    }

    /// Load the signature index, extending or rebuilding it to match the file.
    ///
    /// # Why a second file is allowed here, when the module header argues
    /// against one
    ///
    /// That argument is about a **progress record** — a file that says which
    /// segments are done. A second one of those can survive a crash while
    /// disagreeing with what was actually embedded, and then a sefer is reported
    /// as covered when it is not.
    ///
    /// This is not that. A signature is a pure function of a vector, so the
    /// sidecar is a *cache* of an answer `vectors.bin` already contains, and it
    /// carries the one number that makes disagreement impossible to miss:
    /// `covers`, the byte length of `vectors.bin` these signatures were built
    /// from. The file is append-only, so:
    ///
    /// - `covers` equals the length → the index is complete, read it.
    /// - `covers` is less → the tail is new; sign **only the tail** and append.
    ///   This is the ordinary case after a job runs, and it costs the bytes that
    ///   were just written.
    /// - `covers` is more, or the header names another model, another width or
    ///   another version → it is not about this file; build it again.
    ///
    /// There is no state in which a stale sidecar is used, and none in which one
    /// costs an answer: the worst outcome is [`Vectors::nearest`] reading the
    /// whole file, which is what it did before this existed.
    fn signatures(&mut self, path: &Path) -> Result<(), VectorError> {
        // A store made by another model reads nothing and ranks nothing.
        if self.made_by.is_some() || self.at.is_empty() {
            return Ok(());
        }
        let Ok(meta) = std::fs::metadata(path) else {
            return Ok(());
        };
        // The torn tail is not a record and must not be signed or counted as
        // covered — it will be overwritten by the next append.
        let whole = meta.len().saturating_sub(self.torn);
        let side = Self::sidecar(path);

        let mut covers = 0u64;
        let mut held: Vec<(u64, crate::signature::Signature)> = Vec::new();
        if let Ok(bytes) = std::fs::read(&side) {
            let head = self.sidecar_header();
            if bytes.len() >= head.len() + 8 && bytes[..head.len()] == head[..] {
                let mut eight = [0u8; 8];
                eight.copy_from_slice(&bytes[head.len()..head.len() + 8]);
                let said = u64::from_le_bytes(eight);
                let entries = &bytes[head.len() + 8..];
                let stride = 8 + crate::signature::BYTES;
                // A sidecar longer than its own `covers` claims, or one whose
                // entries do not divide evenly, is a torn write of a cache: it
                // costs nothing to throw away and everything to trust.
                if said <= whole && entries.len() % stride == 0 {
                    covers = said;
                    for entry in entries.chunks_exact(stride) {
                        let mut eight = [0u8; 8];
                        eight.copy_from_slice(&entry[..8]);
                        let mut sig = [0u8; crate::signature::BYTES];
                        sig.copy_from_slice(&entry[8..]);
                        held.push((
                            u64::from_le_bytes(eight),
                            crate::signature::Signature::from_bytes(&sig),
                        ));
                    }
                }
            }
        }

        let fresh = covers == 0;
        let (mut grown, reached) = if covers < whole {
            self.sign_from(path, covers.max(self.header().len() as u64), whole)?
        } else {
            (Vec::new(), covers)
        };

        if !grown.is_empty() || fresh {
            self.write_sidecar(&side, &held, &grown, reached)?;
        }
        held.append(&mut grown);
        // Only the winning record of an id is worth a signature; a superseded
        // one is never a candidate, and holding it would put a stale vector on
        // a shortlist.
        let winning: std::collections::HashSet<u64> = self.at.values().copied().collect();
        self.sig = held
            .into_iter()
            .filter(|(at, _)| winning.contains(at))
            .collect();
        Ok(())
    }

    /// Sign every record from `from` up to `upto`, reading the payloads.
    ///
    /// The one full read of the file this index costs, paid once and then only
    /// over whatever a job appended since.
    fn sign_from(
        &self,
        path: &Path,
        from: u64,
        upto: u64,
    ) -> Result<(Vec<(u64, crate::signature::Signature)>, u64), VectorError> {
        let planes = crate::signature::projection(self.dims);
        let file = std::fs::File::open(path).map_err(VectorError::io(path))?;
        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(from))
            .map_err(VectorError::io(path))?;

        let mut out = Vec::new();
        let mut at = from;
        let payload = self.dims as u64 * 4;
        let mut two = [0u8; 2];
        while at < upto {
            let mut tag = [0u8; 1];
            match reader.read_exact(&mut tag) {
                Ok(()) if tag[0] == RECORD => {}
                _ => break,
            }
            if reader.read_exact(&mut two).is_err() {
                break;
            }
            let mut id = vec![0u8; usize::from(u16::from_le_bytes(two))];
            if reader.read_exact(&mut id).is_err() {
                break;
            }
            let mut bytes = vec![0u8; self.dims * 4];
            if reader.read_exact(&mut bytes).is_err() {
                break;
            }
            let vector: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|four| f32::from_le_bytes([four[0], four[1], four[2], four[3]]))
                .collect();
            out.push((at, planes.sign(&vector)));
            at += 1 + 2 + id.len() as u64 + payload;
        }
        Ok((out, at))
    }

    /// Write the sidecar out whole.
    ///
    /// Whole rather than appended, because it is small (40 bytes a segment) and
    /// because a torn append would be a cache that has to be validated a second
    /// way. Trouble writing it is trouble, not a fault: the caller turns it into
    /// a line and the store ranks the slow way.
    fn write_sidecar(
        &self,
        side: &Path,
        held: &[(u64, crate::signature::Signature)],
        grown: &[(u64, crate::signature::Signature)],
        covers: u64,
    ) -> Result<(), VectorError> {
        if let Some(dir) = side.parent() {
            std::fs::create_dir_all(dir).map_err(VectorError::io(dir))?;
        }
        let mut out = self.sidecar_header();
        out.extend_from_slice(&covers.to_le_bytes());
        for (at, sig) in held.iter().chain(grown) {
            out.extend_from_slice(&at.to_le_bytes());
            out.extend_from_slice(&sig.to_bytes());
        }
        std::fs::write(side, &out).map_err(VectorError::io(side))
    }

    /// One record, read by where it starts.
    ///
    /// What the shortlist is scored with. The offsets come from
    /// [`Vectors::at`], which is built at open, so this is a seek and one
    /// record rather than a walk.
    fn record_at(&self, at: u64) -> Result<Option<Vector>, VectorError> {
        let Some(path) = &self.path else {
            return Ok(None);
        };
        let mut file = std::fs::File::open(path).map_err(VectorError::io(path))?;
        file.seek(SeekFrom::Start(at))
            .map_err(VectorError::io(path))?;
        let mut reader = BufReader::new(file);
        let mut tag = [0u8; 1];
        if reader.read_exact(&mut tag).is_err() || tag[0] != RECORD {
            return Ok(None);
        }
        let mut two = [0u8; 2];
        if reader.read_exact(&mut two).is_err() {
            return Ok(None);
        }
        let mut id = vec![0u8; usize::from(u16::from_le_bytes(two))];
        if reader.read_exact(&mut id).is_err() {
            return Ok(None);
        }
        let mut bytes = vec![0u8; self.dims * 4];
        if reader.read_exact(&mut bytes).is_err() {
            return Ok(None);
        }
        let Ok(id) = String::from_utf8_lossy(&id).parse::<SegmentId>() else {
            return Ok(None);
        };
        Ok(Some(Vector {
            id,
            vector: bytes
                .chunks_exact(4)
                .map(|four| f32::from_le_bytes([four[0], four[1], four[2], four[3]]))
                .collect(),
        }))
    }

    /// The bytes every file of this store's model begins with.
    fn header(&self) -> Vec<u8> {
        let mut header = MAGIC.to_vec();
        let dims = u32::try_from(self.dims).unwrap_or(0);
        header.extend_from_slice(&dims.to_le_bytes());
        let name = self.fingerprint.as_bytes();
        let len = u16::try_from(name.len()).unwrap_or(0);
        header.extend_from_slice(&len.to_le_bytes());
        header.extend_from_slice(&name[..len as usize]);
        header
    }

    /// Read the ids and where they are, skipping the vectors themselves.
    fn read(&mut self, path: &Path) -> Result<(), VectorError> {
        let Ok(file) = std::fs::File::open(path) else {
            // No file is not a fault: it is a sefer nobody has embedded.
            return Ok(());
        };
        let length = file.metadata().map_err(VectorError::io(path))?.len();
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; MAGIC.len()];
        if reader.read_exact(&mut magic).is_err() || &magic != MAGIC {
            return Err(VectorError::NotAStore {
                path: path.display().to_string(),
            });
        }
        let mut four = [0u8; 4];
        let mut two = [0u8; 2];
        reader
            .read_exact(&mut four)
            .map_err(VectorError::io(path))?;
        let dims = usize::try_from(u32::from_le_bytes(four)).unwrap_or(0);
        reader.read_exact(&mut two).map_err(VectorError::io(path))?;
        let mut name = vec![0u8; usize::from(u16::from_le_bytes(two))];
        reader
            .read_exact(&mut name)
            .map_err(VectorError::io(path))?;
        let made_by = String::from_utf8_lossy(&name).into_owned();

        // Two different models, or the same model at two widths, are the same
        // refusal: the numbers would line up and mean nothing.
        if made_by != self.fingerprint || dims != self.dims {
            self.made_by = Some(made_by);
            return Ok(());
        }

        let mut at = self.header().len() as u64;
        let payload = self.dims as u64 * 4;
        loop {
            let mut tag = [0u8; 1];
            match reader.read_exact(&mut tag) {
                Ok(()) if tag[0] == RECORD => {}
                // A clean end, or a tail that is not a record. The second is a
                // crash mid-append, and it is counted.
                Ok(()) => {
                    self.torn = length.saturating_sub(at);
                    break;
                }
                Err(_) => break,
            }
            if reader.read_exact(&mut two).is_err() {
                self.torn = length.saturating_sub(at);
                break;
            }
            let mut id = vec![0u8; usize::from(u16::from_le_bytes(two))];
            if reader.read_exact(&mut id).is_err() {
                self.torn = length.saturating_sub(at);
                break;
            }
            let after = at + 1 + 2 + id.len() as u64 + payload;
            if after > length {
                self.torn = length.saturating_sub(at);
                break;
            }
            // `seek_relative`, not `seek`. This pass exists to read the ids
            // and **skip the vectors**, and `BufReader::seek` throws the whole
            // buffer away on every call — so skipping a 768-float payload
            // discarded the 8 KB that had just been read, and the pass that was
            // meant to avoid reading the file read all of it, one syscall per
            // record. `seek_relative` keeps the buffer when the target is
            // inside it, which for a payload this size it usually is.
            reader
                .seek_relative(i64::try_from(payload).unwrap_or(i64::MAX))
                .map_err(VectorError::io(path))?;

            let Ok(id) = String::from_utf8_lossy(&id).parse::<SegmentId>() else {
                // One unreadable id costs one segment, which comes back round
                // the queue. It does not cost the file.
                at = after;
                continue;
            };
            // Last record for an id wins: this is a log, not a table.
            if self.at.insert(id.clone(), at).is_none() {
                self.order.push(id);
            }
            at = after;
        }
        Ok(())
    }

    /// Append one record, and say where it landed.
    fn append(&self, path: &Path, id: &SegmentId, vector: &[f32]) -> Result<u64, VectorError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(VectorError::io(dir))?;
        }
        let fresh = !path.is_file();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(VectorError::io(path))?;
        if fresh {
            file.write_all(&self.header())
                .map_err(VectorError::io(path))?;
        }
        let at = file.metadata().map_err(VectorError::io(path))?.len();

        let name = id.to_string();
        let mut record = Vec::with_capacity(3 + name.len() + vector.len() * 4);
        record.push(RECORD);
        record.extend_from_slice(&u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
        record.extend_from_slice(name.as_bytes());
        for value in vector {
            record.extend_from_slice(&value.to_le_bytes());
        }
        file.write_all(&record).map_err(VectorError::io(path))?;
        Ok(at)
    }
}

/// The records of a file, one at a time.
struct Records<'a> {
    reader: Option<(BufReader<std::fs::File>, PathBuf)>,
    dims: usize,
    at: u64,
    /// Where each id's winning record is, so a superseded one is skipped.
    newest: HashMap<&'a SegmentId, u64>,
}

impl Iterator for Records<'_> {
    type Item = Result<Vector, VectorError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (reader, path) = self.reader.as_mut()?;
        loop {
            let mut tag = [0u8; 1];
            match reader.read_exact(&mut tag) {
                Ok(()) if tag[0] == RECORD => {}
                _ => return None,
            }
            let mut two = [0u8; 2];
            if reader.read_exact(&mut two).is_err() {
                return None;
            }
            let mut id = vec![0u8; usize::from(u16::from_le_bytes(two))];
            if reader.read_exact(&mut id).is_err() {
                return None;
            }
            let mut bytes = vec![0u8; self.dims * 4];
            if reader.read_exact(&mut bytes).is_err() {
                return None;
            }
            let at = self.at;
            self.at = at + 1 + 2 + id.len() as u64 + (self.dims as u64) * 4;

            let Ok(id) = String::from_utf8_lossy(&id).parse::<SegmentId>() else {
                continue;
            };
            // A segment embedded twice has two records and one vector.
            if self.newest.get(&id).copied() != Some(at) {
                continue;
            }
            let vector = bytes
                .chunks_exact(4)
                .map(|four| f32::from_le_bytes([four[0], four[1], four[2], four[3]]))
                .collect();
            let _ = path;
            return Some(Ok(Vector { id, vector }));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-lane-vectors-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn id(n: u32) -> SegmentId {
        format!("girsa:bavli/berakhot/2a:{n}#{n}")
            .parse()
            .expect("an id")
    }

    fn unit(a: f32, b: f32) -> Vec<f32> {
        let norm = (a * a + b * b).sqrt().max(1e-12);
        vec![a / norm, b / norm]
    }

    #[test]
    fn a_vector_survives_being_written_down_and_read_back() {
        let dir = scratch("round-trip");
        let (mut store, trouble) = Vectors::open(&dir, "bavli/berakhot", "abc", 2);
        assert!(trouble.is_empty(), "{trouble:?}");
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");
        store.record(&id(2), &unit(0.0, 1.0)).expect("writes");

        let (again, trouble) = Vectors::open(&dir, "bavli/berakhot", "abc", 2);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(again.len(), 2);
        assert!(again.has(&id(1)) && again.has(&id(2)));
        let near = again.nearest(&unit(1.0, 0.1), 2).expect("ranks");
        assert_eq!(near[0].0, id(1));
        assert!(near[0].1 > near[1].1);
    }

    #[test]
    fn embedding_a_segment_twice_leaves_one_vector() {
        // The same argument `girsa-scan`'s pages.jsonl makes: a log keyed by
        // its subject cannot double on a second run.
        let dir = scratch("twice");
        let (mut store, _) = Vectors::open(&dir, "x", "abc", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");
        store.record(&id(1), &unit(0.0, 1.0)).expect("writes");

        let (again, trouble) = Vectors::open(&dir, "x", "abc", 2);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert_eq!(again.len(), 1);
        let all: Vec<Vector> = again
            .all()
            .expect("reads")
            .collect::<Result<_, _>>()
            .expect("reads");
        assert_eq!(all.len(), 1, "the newest record, and only it");
        assert!(all[0].vector[1] > 0.9, "the second one won: {:?}", all[0]);
    }

    #[test]
    fn vectors_made_by_another_model_are_refused_by_both_names_and_not_deleted() {
        // The defect this file format exists to prevent. Two spaces, the same
        // arithmetic, a ranked list that looks exactly like a good one.
        let dir = scratch("other-model");
        let (mut store, _) = Vectors::open(&dir, "x", "berel-3", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");

        let (other, trouble) = Vectors::open(&dir, "x", "something-else", 2);
        assert_eq!(other.made_by_something_else(), Some("berel-3"));
        assert!(trouble.is_empty(), "a mismatch is a state, not trouble");
        assert_eq!(other.len(), 0, "nothing is read out of it");
        let said = other
            .nearest(&unit(1.0, 0.0), 5)
            .expect_err("refuses")
            .to_string();
        assert!(
            said.contains("berel-3") && said.contains("something-else"),
            "{said}"
        );

        // And the file is still there: pointing the setting back at the model
        // that made them costs nothing.
        let (back, _) = Vectors::open(&dir, "x", "berel-3", 2);
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn the_same_model_at_a_different_width_is_also_another_model() {
        let dir = scratch("width");
        let (mut store, _) = Vectors::open(&dir, "x", "berel-3", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");
        let (other, _) = Vectors::open(&dir, "x", "berel-3", 4);
        assert!(other.made_by_something_else().is_some());
        assert_eq!(other.len(), 0);
    }

    #[test]
    fn restarting_clears_the_file_and_only_when_it_is_asked_for() {
        let dir = scratch("restart");
        let (mut store, _) = Vectors::open(&dir, "x", "old", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");

        let (mut fresh, _) = Vectors::open(&dir, "x", "new", 2);
        assert!(fresh.record(&id(1), &unit(1.0, 0.0)).is_err());
        fresh.restarted().expect("restarts");
        fresh.record(&id(2), &unit(0.0, 1.0)).expect("writes");

        let (again, trouble) = Vectors::open(&dir, "x", "new", 2);
        assert!(trouble.is_empty(), "{trouble:?}");
        assert!(again.made_by_something_else().is_none());
        assert_eq!(again.len(), 1);
        assert!(again.has(&id(2)) && !again.has(&id(1)));
    }

    #[test]
    fn a_torn_tail_costs_its_own_segments_and_says_how_many_bytes() {
        // A crash mid-append. The segments in the tail have to come back round
        // the queue — a sefer that lost four vectors silently would be counted
        // as covered.
        let dir = scratch("torn");
        let (mut store, _) = Vectors::open(&dir, "x", "abc", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("writes");
        store.record(&id(2), &unit(0.0, 1.0)).expect("writes");

        let path = Vectors::path_in(&dir, "x");
        let mut bytes = std::fs::read(&path).expect("reads");
        bytes.truncate(bytes.len() - 5);
        std::fs::write(&path, &bytes).expect("writes");

        let (again, trouble) = Vectors::open(&dir, "x", "abc", 2);
        assert_eq!(
            again.len(),
            1,
            "the whole record survives, the torn one does not"
        );
        assert!(again.has(&id(1)) && !again.has(&id(2)));
        assert_eq!(trouble.len(), 1, "{trouble:?}");
        assert!(trouble[0].contains("ignored"), "{trouble:?}");
    }

    #[test]
    fn a_file_that_is_not_a_store_is_named_rather_than_parsed() {
        let dir = scratch("nonsense");
        let path = Vectors::path_in(&dir, "x");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(&path, b"this is not a lane store at all").expect("writes");
        let (store, trouble) = Vectors::open(&dir, "x", "abc", 2);
        assert_eq!(store.len(), 0);
        assert_eq!(trouble.len(), 1, "{trouble:?}");
        assert!(trouble[0].contains("not a lane store"), "{trouble:?}");
    }

    #[test]
    fn a_vector_of_the_wrong_width_is_refused_rather_than_padded() {
        let mut store = Vectors::nowhere("abc", 2);
        assert!(store.record(&id(1), &[1.0, 0.0, 0.0]).is_err());
    }

    #[test]
    fn the_top_ten_is_the_top_ten_a_full_sort_would_have_given() {
        // `nearest` keeps a bounded list rather than sorting 18,120 rows to
        // read ten. The property that has to survive is the *stability* the
        // truncated sort had: equal scores stay in the order the file holds
        // them, which is the order they were embedded in, which is reading
        // order.
        let dir = std::env::temp_dir().join("girsa-vectors-topn");
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Vectors::open(&dir, "x", "m", 2).0;

        // Deliberately with ties, and written in reading order.
        let scores: [f32; 8] = [0.1, 0.9, 0.5, 0.9, 0.3, 0.5, 0.9, 0.2];
        for (n, score) in scores.iter().enumerate() {
            let id = SegmentId::new(
                "x",
                vec![(n + 1).to_string()],
                girsa_corpus::segment::Ordinal::root(u32::try_from(n + 1).expect("a small number")),
            );
            store.record(&id, &[*score, 0.0]).expect("it is written");
        }
        let store = Vectors::open(&dir, "x", "m", 2).0;

        // What a full sort would have said.
        let mut all: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
        all.sort_by(|a, b| b.1.total_cmp(&a.1));

        for most in 1..=scores.len() {
            let got = store.nearest(&[1.0, 0.0], most).expect("it ranks");
            let want: Vec<String> = all
                .iter()
                .take(most)
                .map(|(n, _)| format!("girsa:x/{}#{}", n + 1, n + 1))
                .collect();
            let got: Vec<String> = got.into_iter().map(|(id, _)| id.to_string()).collect();
            assert_eq!(got, want, "top {most} disagrees with a full sort");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A deterministic spread of unit vectors, for the index tests.
    fn spread(n: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
        let mut state = seed;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as f32 / (1u64 << 31) as f32) - 0.5
        };
        (0..n)
            .map(|_| {
                let raw: Vec<f32> = (0..dims).map(|_| next()).collect();
                let norm = raw.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
                raw.into_iter().map(|v| v / norm).collect()
            })
            .collect()
    }

    /// A store with `how_many` deterministic vectors in it, and what went in.
    fn filled(dir: &std::path::Path, how_many: usize, dims: usize) -> Vec<Vec<f32>> {
        let vectors = spread(how_many, dims, 20_260_809);
        let (mut store, _) = Vectors::open(dir, "x", "m", dims);
        for (n, v) in vectors.iter().enumerate() {
            store
                .record(&id(u32::try_from(n).expect("a small number")), v)
                .expect("it is written");
        }
        vectors
    }

    #[test]
    fn a_small_store_is_read_whole_and_says_so() {
        let dir = scratch("small-is-whole");
        filled(&dir, 64, 8);
        let store = Vectors::open(&dir, "x", "m", 8).0;
        let ranked = store
            .nearest_reporting(&spread(1, 8, 7)[0], 5)
            .expect("it ranks");
        assert!(ranked.whole, "64 vectors is under the threshold");
        assert_eq!(ranked.read, 64, "every record read");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_big_store_answers_from_the_index_and_says_so() {
        // The whole point of the change: above the threshold a query reads a
        // shortlist rather than the file. `read` is the number the 9 August
        // report says is stated nowhere.
        let dir = scratch("big-is-indexed");
        let how_many = EXACT_UNDER + 500;
        filled(&dir, how_many, 8);
        let store = Vectors::open(&dir, "x", "m", 8).0;
        let ranked = store
            .nearest_reporting(&spread(1, 8, 11)[0], 10)
            .expect("it ranks");
        assert!(!ranked.whole, "it answered from a shortlist");
        assert!(
            ranked.read <= (10 * OVERSAMPLE).max(SHORTLIST_LEAST),
            "read {} of {how_many}",
            ranked.read
        );
        assert_eq!(ranked.best.len(), 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_index_finds_what_reading_the_whole_file_finds() {
        // The claim that makes a shortlist usable at all, checked against an
        // exhaustive dot product computed here, over a store big enough to be
        // indexed.
        let dir = scratch("index-agrees");
        let vectors = filled(&dir, EXACT_UNDER + 1_000, 16);
        let store = Vectors::open(&dir, "x", "m", 16).0;
        let query = &spread(1, 16, 99)[0];

        let mut exact: Vec<(usize, f32)> = vectors
            .iter()
            .enumerate()
            .map(|(n, v)| (n, query.iter().zip(v).map(|(a, b)| a * b).sum()))
            .collect();
        exact.sort_by(|a, b| b.1.total_cmp(&a.1));

        let ranked = store.nearest_reporting(query, 5).expect("it ranks");
        assert!(!ranked.whole, "the fixture has to be big enough to index");
        let got: Vec<String> = ranked.best.iter().map(|(i, _)| i.to_string()).collect();
        let want: Vec<String> = exact
            .iter()
            .take(5)
            .map(|(n, _)| id(u32::try_from(*n).expect("a small number")).to_string())
            .collect();
        assert_eq!(got, want, "the shortlist lost one of the true five");
        // And the scores are exact scores, never the estimate.
        for ((_, score), (_, want)) in ranked.best.iter().zip(&exact) {
            assert!((score - want).abs() < 1e-5, "{score} is not {want}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sidecar_grows_with_the_file_rather_than_being_rebuilt() {
        // `covers` is the whole validation: the file is append-only, so a
        // sidecar that covers less of it needs the tail signed and nothing else.
        let dir = scratch("sidecar-grows");
        filled(&dir, 40, 8);
        let path = Vectors::path_in(&dir, "x");
        let side = Vectors::sidecar(&path);
        let _ = Vectors::open(&dir, "x", "m", 8);
        let first = std::fs::metadata(&side).expect("a sidecar").len();

        let (mut store, _) = Vectors::open(&dir, "x", "m", 8);
        for (n, v) in spread(4, 8, 3).iter().enumerate() {
            store
                .record(&id(u32::try_from(100 + n).expect("a small number")), v)
                .expect("it is written");
        }
        let store = Vectors::open(&dir, "x", "m", 8).0;
        let second = std::fs::metadata(&side).expect("a sidecar").len();
        assert_eq!(
            second - first,
            4 * (8 + crate::signature::BYTES) as u64,
            "four more signatures and nothing else"
        );
        assert_eq!(store.sig.len(), 44, "one per segment");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sidecar_from_another_model_is_thrown_away_rather_than_believed() {
        // The refusal the vectors file itself makes, one layer down: signatures
        // made in another model's space would shortlist noise, and nothing
        // downstream could tell.
        let dir = scratch("sidecar-other-model");
        filled(&dir, 40, 8);
        let _ = Vectors::open(&dir, "x", "m", 8);
        let mine =
            std::fs::read(Vectors::sidecar(&Vectors::path_in(&dir, "x"))).expect("a sidecar");

        let elsewhere = scratch("sidecar-other-model-planted");
        let (mut other, _) = Vectors::open(&elsewhere, "y", "n", 8);
        for (n, v) in spread(40, 8, 5).iter().enumerate() {
            other
                .record(&id(u32::try_from(n).expect("a small number")), v)
                .expect("it is written");
        }
        let planted = Vectors::sidecar(&Vectors::path_in(&elsewhere, "y"));
        std::fs::write(&planted, &mine).expect("it is planted");

        let reopened = Vectors::open(&elsewhere, "y", "n", 8).0;
        let head = reopened.sidecar_header();
        let bytes = std::fs::read(&planted).expect("a sidecar");
        assert_eq!(
            &bytes[..head.len()],
            &head[..],
            "it built its own rather than reading a stranger's"
        );
        assert_eq!(reopened.sig.len(), 40);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn a_superseded_record_is_not_a_candidate() {
        // A segment embedded twice has two records and one vector. The index
        // holds the winning one only, or a shortlist could carry a vector the
        // store does not consider current.
        let dir = scratch("superseded");
        let (mut store, _) = Vectors::open(&dir, "x", "m", 2);
        store.record(&id(1), &unit(1.0, 0.0)).expect("first");
        store.record(&id(1), &unit(0.0, 1.0)).expect("again");
        store.record(&id(2), &unit(0.5, 0.5)).expect("other");
        let store = Vectors::open(&dir, "x", "m", 2).0;
        assert_eq!(store.sig.len(), 2, "three records, two segments");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
