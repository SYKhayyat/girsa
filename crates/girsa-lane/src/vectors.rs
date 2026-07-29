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
}

/// A vector, and the segment it is of.
#[derive(Debug, Clone)]
pub struct Vector {
    pub id: SegmentId,
    pub vector: Vec<f32>,
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
        };
        let mut trouble = Vec::new();
        if let Err(e) = store.read(&path) {
            trouble.push(e.to_string());
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
    /// It reads the file. That is the trade this store makes on purpose: the
    /// selection a reader is allowed to make goes up to all 5,000,545 segments,
    /// and holding that in memory is 15 GB. Reading it is bounded by the disk
    /// and by what the reader chose to embed, which is the number they can see
    /// and change.
    ///
    /// # Errors
    ///
    /// If the file will not read, or was made by another model.
    pub fn nearest(
        &self,
        query: &[f32],
        most: usize,
    ) -> Result<Vec<(SegmentId, f32)>, VectorError> {
        if let Some(made_by) = &self.made_by {
            return Err(VectorError::OtherModel {
                made_by: made_by.clone(),
                configured: self.fingerprint.clone(),
            });
        }
        if most == 0 || query.len() != self.dims || self.order.is_empty() {
            return Ok(Vec::new());
        }
        let mut best: Vec<(SegmentId, f32)> = Vec::new();
        for vector in self.all()? {
            let vector = vector?;
            let near: f32 = query.iter().zip(&vector.vector).map(|(a, b)| a * b).sum();
            best.push((vector.id, near));
        }
        // Stable, so equal scores stay in the order the file has them, which is
        // the order they were embedded in, which is reading order.
        best.sort_by(|a, b| b.1.total_cmp(&a.1));
        best.truncate(most);
        Ok(best)
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
            reader
                .seek(SeekFrom::Start(after))
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
}
