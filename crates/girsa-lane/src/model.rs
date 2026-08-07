//! The model — side-loaded, never fetched, and named in everything it writes.
//!
//! spec.md §9.9, ruled in §16 #20: **Girsa never downloads a model.** You point
//! a setting at a directory you obtained yourself and the lane reads it from
//! there. That is not squeamishness about bandwidth — it is the only shape that
//! keeps two rules that were settled long before this work order:
//!
//! - **§14, offline is the product.** The one sanctioned network use is a corpus
//!   update. A search box that reached out for 738 MB the first time somebody
//!   asked it a question would have quietly made Girsa an online application.
//! - **§13 and BUILDER.md T7, the licence line.** This repository is MIT OR
//!   Apache-2.0 because it shares crates with Ksav. A model that lives on the
//!   reader's disk carries its own terms on the reader's disk; a model vendored
//!   into the repository would carry them into the repository.
//!
//! It is the same arrangement `girsa-scan`'s OCR engine is under, for the same
//! reason and with the same consequence: **absence is a state with a name**, not
//! a button that does nothing. See [`ModelError::NotConfigured`].
//!
//! # Which model, and what its licence actually is
//!
//! BUILDER.md W30 says to verify BEREL's licence before writing a line, because
//! spec.md §9.4's candidate table called it *unrestricted* while the README
//! warned it carries its own terms — and those are not the same claim.
//!
//! Checked, on 29 July 2026, three ways: the model card, its YAML frontmatter,
//! and the Hub API's own metadata for `dicta-il/BEREL_2.0` (which redirects to
//! **BEREL 3.0**). All three say **`apache-2.0`**, with a request to cite the
//! paper. That is one of this repository's own two licences, so a reader
//! side-loading it is clean, and Girsa vendoring nothing keeps it a question
//! about the reader's disk rather than about this tree either way.
//!
//! # Nothing here is BEREL-specific
//!
//! What is read is a BERT: a `config.json`, a `tokenizer.json`, and weights in
//! `model.safetensors`. Any encoder in that shape works, and the fingerprint
//! (below) is what keeps two of them from being mixed. BEREL is the one this
//! was measured against because it is the one trained on the right register —
//! ~220M words of rabbinic Hebrew and Aramaic — and a modern-Hebrew encoder on
//! a Rishon is the mistake spec.md §9.4 already catalogued for morphology.
//!
//! # What it actually does, measured
//!
//! BUILDER.md W30 is accepted on *a query that shares no words with its target
//! finds it*, and that is a claim about a **model**, not about code. So it was
//! measured the way W26 measured tesseract, and the shape of the feature follows
//! the numbers rather than the hope.
//!
//! Hilchos Tefillah — `mishneh-torah/prayer-and-the-priestly-blessing`, 240
//! se'ifim, fully menukad — embedded with BEREL 3.0 and asked 22 questions with
//! a known right answer. The *over* column is how many words the query shares
//! with its target after nikud is stripped and nothing else, which is exactly
//! what Torat Emet would have matched on. Reproduce it with
//! `cargo run --release -p girsa-lane --example measure`.
//!
//! **Asked as a question** — *how late may one daven shacharis?* — 12 pairs,
//! 5 of them sharing no word at all:
//!
//! | | rank 1 | top 5 | top 10 |
//! |---|---|---|---|
//! | 5 pairs sharing no word | 1 | 1 | 1 |
//!
//! One in five. The four that missed landed at 23, 24, 48 and 66 of 240.
//!
//! **Asked as a half-remembered statement** — *I think it says the drunk may not
//! daven because he has no kavanah* — 10 pairs:
//!
//! | | rank 1 | top 5 | top 10 |
//! |---|---|---|---|
//! | all 10 pairs | 8 | 9 | 10 |
//! | the 2 sharing no word at all | 0 | 0 | 1 |
//!
//! **Every one of the ten is in the top 16 of 240, and eight are first.** The
//! two that share nothing at all rank 9 and 16. And the eight that rank first
//! share only function words — `לא`, `לו`, `מי`, `על`, `עד`, `אחת` — the kind
//! of overlap a literal search drowns in rather than finds anything with.
//!
//! # What that decided
//!
//! **1 · This answers spec.md §9.9's sentence and not the other one.** §9.9 asks
//! for *"I remember a Rishon who says something like this but not the words"* —
//! a half-remembered **claim**, which is a near-symmetric retrieval task. A
//! question about a passage is an asymmetric one, and BEREL was never trained
//! for it: it is a masked-language model, not a sentence encoder, and nothing in
//! its training gave it a similarity objective. So the lane's box asks for a
//! line as you remember it, and it does not pretend to answer questions.
//!
//! **2 · Mean-centring was tried and thrown out.** Every sentence a raw BERT
//! produces sits in a narrow cone, so the standard repair is to subtract the
//! mean of the space, and the standard repair **made it worse here** — the
//! *centred* column above moved 24→40, 97→123, 9→24. It is measured in
//! `examples/measure.rs` and it is not built. A plausible improvement that does
//! not survive measurement is the exact thing this project's search section
//! exists to refuse.
//!
//! **3 · The model being side-loaded is what makes this safe to ship.** Nothing
//! here is BEREL-specific: any BERT-shaped encoder in a directory works, and the
//! [`fingerprint`] keeps two of them from being mixed. The day a contrastively
//! trained rabbinic-Hebrew sentence encoder exists, a reader points the setting
//! at it and re-embeds — one pass, nothing to migrate, no release needed. That
//! is the same *make the decision reversible rather than permanent* move W26
//! made for OCR, and for the same reason.
//!
//! **4 · Throughput is the reason you choose the corpus.** 4.5 segments a second
//! on one CPU, release build, batches of 16 — [`SEGMENTS_A_SECOND`]. That is 54
//! seconds for Hilchos Tefillah and **about thirteen days for all 5,000,545
//! segments** — which is why §16 #20 makes the corpus a choice and why
//! [`crate::job`] is resumable. The thirteen days are now *in the sentence that
//! offers the whole shelf*: `Coverage::said` spends this constant, because
//! `Chosen::everything()` is a first-class standing choice and until 6 August
//! 2026 the line that made the offer did not mention what it costs.
//!
//! **5 · Everything above is at n=240, and the offer is at n=5,000,545.** This
//! is the limit the crate did not name, and it names its others with real
//! rigour. Hilchos Tefillah is 240 se'ifim; the shelf is five million segments.
//! *Top-16 of 240* and *a 0.11 cosine margin* — 0.74 for the right answer
//! against 0.63 for unrelated se'ifim — are a different claim at each size. **At
//! 240 the tail is empty. At 5,000,545 the tail is the answer set**, and there
//! is nothing in this measurement that says how a 0.11 margin behaves against
//! twenty thousand times as many candidates. It may hold. It has not been
//! looked at.
//!
//! So the honest thing is said out loud rather than assumed either way, in
//! [`crate::MEASURED`], drawn under every answer the window shows. Re-running
//! `examples/measure.rs` over ~50,000 segments would replace that sentence with
//! a number, and that is the one afternoon this feature still owes.
//!
//! # The forward pass is `candle-transformers`, and used not to be
//!
//! This module carried **140 lines of hand-written BERT** — embeddings, an
//! encoder block, the attention — against `candle-core` and `candle-nn`, while
//! `candle-transformers` was absent from the manifest with nothing anywhere
//! saying why. Same crate family, same version, same two licences, and the
//! reference implementation of exactly this network.
//!
//! It went, and the going was **checked rather than asserted**, because nothing
//! in this crate tested that the forward pass computed anything at all: every
//! other test here is about a refusal — a missing file, a config that will not
//! run, a fingerprint. So a whole eight-dimensional BERT is now built from one
//! seeded generator in `the_forward_pass_produces_the_vector_it_produced_before`,
//! and the vector it produces is compared against the one the hand-written pass
//! produced. **The largest component moved by 8e-8** — one ulp of f32, from
//! `(a + b) + c` against `a + (b + c)` in the embedding sum.
//!
//! That number is the whole argument. A reader who has spent days embedding a
//! shelf still has the same vectors, and [`fingerprint`] does not have to move —
//! which it otherwise would have had to, because a fingerprint that says *which
//! model* and not *which arithmetic* would have let two implementations' vectors
//! into one store and produced a ranking that looked exactly like a good one.
//!
//! What is **not** delegated is the pooling. `BertModel` returns token states;
//! mean-pooling them over the real tokens and normalising is Girsa's decision
//! and stays here — see [`Model::run`].
//!
//! # The tokenizer is part of the model
//!
//! This project normally reimplements rather than copies, and the normalizer in
//! [`girsa_hebrew`] is the one thing every Hebrew comparison in Girsa routes
//! through. **The lane does not use it.** BEREL's `tokenizer.json` carries an
//! `NFD → Lowercase → StripAccents` normalizer, a `[א-ת]`-aware regex
//! pre-tokenizer that knows about gershayim, and a 128,000-entry WordPiece
//! vocabulary — and those are not a preprocessing preference, they are *what
//! the weights were trained against*. A word split differently from how it was
//! trained does not fail; it embeds, into the wrong place, silently. So the
//! model's own tokenizer runs, and `girsa-hebrew` stays the rule for literal
//! search, which is where it is the authority.

use std::path::{Path, PathBuf};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{
    BertModel, Config as BertConfig, HiddenAct, PositionEmbeddingType,
};
use serde::Deserialize;
use tokenizers::{Tokenizer, TruncationParams};

/// The three files a model directory has to hold.
///
/// Named as a list so that a directory missing one is refused **with the name
/// of the file**, rather than with whatever error the first thing to touch it
/// happened to raise.
pub const WANTED: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

/// The most tokens of one segment that reach the model.
///
/// BERT's learned position embeddings stop at `max_position_embeddings`, which
/// is 512 in every checkpoint this was tried against, and the model's own
/// config is read rather than this constant — this is only the ceiling on what
/// is asked for. A segment longer than that is **truncated, and the truncation
/// is counted** ([`Embedded::truncated`]): a sefer whose se'ifim run past the
/// window is embedded from its opening, which is a real limitation and not one
/// a reader should have to infer from results that feel off.
pub const MOST_TOKENS: usize = 512;

/// How many segments go through the model at once.
///
/// Batching is most of the throughput — the matmuls are the same size either
/// way and the per-call overhead is not — and this is the size past which the
/// activations stop fitting comfortably beside the weights on an ordinary
/// laptop. The job is resumable at any batch boundary, so this is also the
/// most work a reader can lose by closing the window.
pub const BATCH: usize = 16;

/// Segments a second, measured: one CPU, release build, batches of [`BATCH`].
///
/// The number behind *"about thirteen days for all 5,000,545 segments"* in the
/// module note. It is a constant rather than a sentence because
/// [`crate::Coverage::said`] now spends it: a reader offered
/// [`crate::Chosen::everything`] is being offered thirteen days, and until this
/// existed the sentence that made the offer did not mention them.
///
/// A floor rather than a promise. It is what one ordinary laptop did; a faster
/// machine finishes sooner and nobody is disappointed by that direction.
pub const SEGMENTS_A_SECOND: f64 = 4.5;

/// How long embedding `segments` more of them takes, in the words a sentence
/// wants — `None` when there is nothing left to do.
///
/// Rounded coarsely on purpose. *"About two weeks"* is the decision the reader
/// is making; *"13 days, 4 hours"* is a precision this measurement does not
/// have and would invite somebody to check a clock against.
#[must_use]
pub fn how_long(segments: usize) -> Option<String> {
    if segments == 0 {
        return None;
    }
    let seconds = segments as f64 / SEGMENTS_A_SECOND;
    let (n, unit) = if seconds < 90.0 {
        (seconds.ceil(), "second")
    } else if seconds < 90.0 * 60.0 {
        ((seconds / 60.0).ceil(), "minute")
    } else if seconds < 36.0 * 3600.0 {
        ((seconds / 3600.0).ceil(), "hour")
    } else {
        ((seconds / 86_400.0).ceil(), "day")
    };
    let n = n as u64;
    Some(format!("about {n} {unit}{}", if n == 1 { "" } else { "s" }))
}

/// Why the lane could not embed anything.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// **A state, not a failure.** No model has been pointed at, so the lane is
    /// off — which is what off looks like from the inside, and what the search
    /// header says out loud rather than returning nothing and looking like it
    /// worked (spec.md §9.9).
    #[error(
        "no semantic model is configured — the lane is off. Point it at a model directory you \
         already have (one holding config.json, tokenizer.json and model.safetensors); Girsa \
         downloads nothing"
    )]
    NotConfigured,
    #[error("{dir} is not a model directory — it has no {}", .missing.join(", no "))]
    NotAModel { dir: String, missing: Vec<String> },
    #[error("reading {path}: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Malformed { path: String, message: String },
    /// The directory holds a model this lane cannot run. Said plainly, because
    /// *nothing happened* over a model a reader deliberately went and fetched
    /// is the least useful answer available.
    #[error("{dir}: {why}")]
    Unsupported { dir: String, why: String },
    #[error("the model would not run: {0}")]
    WouldNotRun(String),
}

impl ModelError {
    fn io(path: &Path) -> impl Fn(std::io::Error) -> Self + '_ {
        move |source| Self::Unreadable {
            path: path.display().to_string(),
            source,
        }
    }

    fn bad(path: &Path, message: impl std::fmt::Display) -> Self {
        Self::Malformed {
            path: path.display().to_string(),
            message: message.to_string(),
        }
    }

    fn ran(error: impl std::fmt::Display) -> Self {
        Self::WouldNotRun(error.to_string())
    }
}

/// What came back for one segment.
#[derive(Debug, Clone)]
pub struct Embedded {
    /// Unit length, so nearness is a dot product and nothing downstream has to
    /// remember to divide.
    pub vector: Vec<f32>,
    /// Whether the segment was longer than the model's window and the tail was
    /// not read. Counted rather than logged — see [`MOST_TOKENS`].
    pub truncated: bool,
}

/// Something that can turn a piece of text into a vector.
///
/// A trait for the same reason `girsa_scan::engine::Engine` is one: the machinery
/// around it — what is embedded, what is not, what the coverage line says — is
/// the part that has to be right, and it must be testable without 738 MB of
/// weights on the machine running the tests.
///
/// # `Send + Sync`, and why it is a requirement rather than a convenience
///
/// spec.md §9.9: embedding **never blocks reading**. The only way to mean that
/// is for the job to run on another thread while the window answers, and for
/// both to be looking at the *same* model — 738 MB loaded twice so that a
/// progress bar can move is not a design, it is an apology. So an embedder is
/// shared behind an [`std::sync::Arc`] and has to be safe to share.
pub trait Embedder: Send + Sync {
    /// What made these vectors. Written into the store's header, and refused
    /// against on the way back in — see [`crate::vectors`].
    fn fingerprint(&self) -> &str;

    /// What a reader calls it, for the search header.
    ///
    /// Defaults to the fingerprint, which is what a stub has and all it needs; a
    /// real model says the directory it came out of as well, because *using
    /// 1fd507adf1dd3da6* tells a reader nothing they can act on.
    fn named(&self) -> String {
        self.fingerprint().to_string()
    }

    /// How long a vector is.
    fn dims(&self) -> usize;

    /// Embed a batch.
    ///
    /// Batch-first rather than one-at-a-time-with-a-batch-helper, so there is
    /// one path through the model and a batch of one is not a different code
    /// path from a batch of sixteen.
    ///
    /// # Errors
    ///
    /// If the model will not run. An empty batch is not an error and comes back
    /// empty.
    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedded>, ModelError>;
}

/// A BERT read off the disk.
pub struct Model {
    dir: PathBuf,
    fingerprint: String,
    tokenizer: Tokenizer,
    pad: u32,
    config: Config,
    bert: BertModel,
    device: Device,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("dir", &self.dir)
            .field("fingerprint", &self.fingerprint)
            .field("dims", &self.config.hidden_size)
            .field("layers", &self.config.num_hidden_layers)
            .finish()
    }
}

impl Model {
    /// Read the model in this directory.
    ///
    /// # Errors
    ///
    /// If the directory is not a model directory, if it holds a model of a
    /// shape this cannot run, or if the weights will not load. Every one of
    /// those says which, because a reader who went and downloaded a model is
    /// owed more than *the lane is off*.
    pub fn side_loaded(dir: &Path) -> Result<Self, ModelError> {
        let missing: Vec<String> = WANTED
            .iter()
            .filter(|name| !dir.join(name).is_file())
            .map(|name| (*name).to_string())
            .collect();
        if !missing.is_empty() {
            return Err(ModelError::NotAModel {
                dir: dir.display().to_string(),
                missing,
            });
        }

        let config_path = dir.join("config.json");
        let config_bytes = std::fs::read(&config_path).map_err(ModelError::io(&config_path))?;
        let config: Config =
            serde_json::from_slice(&config_bytes).map_err(|e| ModelError::bad(&config_path, e))?;
        config.check(dir)?;

        let tokenizer_path = dir.join("tokenizer.json");
        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| ModelError::bad(&tokenizer_path, e))?;
        // The window is the model's, not this crate's: a checkpoint with 256
        // positions must not be handed 512 tokens, and one with 1,024 should
        // not be cut at 512 by a constant written here.
        let window = config.max_position_embeddings.min(MOST_TOKENS);
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: window,
                ..TruncationParams::default()
            }))
            .map_err(|e| ModelError::bad(&tokenizer_path, e))?;
        let pad = tokenizer.token_to_id("[PAD]").unwrap_or(0);

        let weights_path = dir.join("model.safetensors");
        let fingerprint = fingerprint(&config_bytes, &weights_path)?;

        let device = Device::Cpu;
        let tensors = candle_core::safetensors::load(&weights_path, &device)
            .map_err(|e| ModelError::bad(&weights_path, e))?;
        // `BertLMHeadModel` checkpoints — BEREL is one — keep the encoder under
        // a `bert.` prefix beside a `cls.` masked-LM head that is no use here.
        // A plain `BertModel` has neither. Both are read, because which one a
        // reader downloaded is not something they chose.
        let prefix = if tensors.contains_key("bert.embeddings.word_embeddings.weight") {
            "bert"
        } else if tensors.contains_key("embeddings.word_embeddings.weight") {
            ""
        } else {
            return Err(ModelError::Unsupported {
                dir: dir.display().to_string(),
                why: "model.safetensors holds no BERT encoder — there is no \
                      embeddings.word_embeddings.weight in it, under `bert.` or otherwise"
                    .to_string(),
            });
        };
        let vb = VarBuilder::from_tensors(tensors, DType::F32, &device);
        let vb = if prefix.is_empty() { vb } else { vb.pp(prefix) };

        let bert = BertModel::load(vb, &config.as_bert()).map_err(ModelError::ran)?;

        Ok(Self {
            dir: dir.to_path_buf(),
            fingerprint,
            tokenizer,
            pad,
            config,
            bert,
            device,
        })
    }

    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// What the setting screen and the coverage line call it: the directory's
    /// own name, and the fingerprint that tells two checkpoints apart.
    #[must_use]
    pub fn named(&self) -> String {
        let name = self.dir.file_name().map_or_else(
            || self.dir.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        format!("{name} ({})", self.fingerprint)
    }

    /// One forward pass over a padded batch, mean-pooled over the real tokens.
    ///
    /// **Mean-pooled, not `[CLS]`-pooled**, and not by preference: BEREL ships
    /// no pooler — there is no `bert.pooler.dense` in its safetensors — so its
    /// `[CLS]` position was never trained to stand for the sentence. Averaging
    /// the token states is what is left, and it is what works.
    fn run(&self, ids: &[Vec<u32>]) -> Result<Vec<Vec<f32>>, candle_core::Error> {
        let batch = ids.len();
        let width = ids.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let mut padded = Vec::with_capacity(batch * width);
        let mut keep = Vec::with_capacity(batch * width);
        for row in ids {
            for at in 0..width {
                match row.get(at) {
                    Some(id) => {
                        padded.push(*id);
                        keep.push(1.0f32);
                    }
                    None => {
                        padded.push(self.pad);
                        keep.push(0.0f32);
                    }
                }
            }
        }

        let ids = Tensor::from_vec(padded, (batch, width), &self.device)?;
        let keep = Tensor::from_vec(keep, (batch, width), &self.device)?;
        // One sentence at a time, so every token is type 0. Built rather than
        // skipped because BERT's type embedding for 0 is not zero.
        let types = Tensor::zeros((batch, width), DType::U32, &self.device)?;

        // Padding is masked out of the attention as well as out of the average.
        // Left in, it would be one more token every real token attends to, and
        // a batch of sixteen would embed a segment differently from a batch of
        // one — which would make the vectors depend on the order the job
        // happened to run in.
        let hidden = self.bert.forward(&ids, &types, Some(&keep))?;

        let weights = keep.reshape((batch, width, 1))?;
        let summed = hidden.broadcast_mul(&weights)?.sum(1)?;
        let counts = weights.sum(1)?.clamp(1.0, f64::INFINITY)?;
        let mean = summed.broadcast_div(&counts)?;
        // Unit length here, once, so nearness downstream is a dot product.
        let norms = mean
            .sqr()?
            .sum_keepdim(1)?
            .sqrt()?
            .clamp(1e-12, f64::INFINITY)?;
        mean.broadcast_div(&norms)?.to_vec2()
    }
}

impl Embedder for Model {
    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn named(&self) -> String {
        Self::named(self)
    }

    fn dims(&self) -> usize {
        self.config.hidden_size
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedded>, ModelError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let window = self.config.max_position_embeddings.min(MOST_TOKENS);
        let mut ids = Vec::with_capacity(texts.len());
        let mut truncated = Vec::with_capacity(texts.len());
        for text in texts {
            let encoded = self
                .tokenizer
                .encode(*text, true)
                .map_err(|e| ModelError::WouldNotRun(e.to_string()))?;
            let row = encoded.get_ids().to_vec();
            truncated.push(row.len() >= window);
            ids.push(row);
        }
        // Grouped by length before the forward pass, and put back in the order
        // they came in.
        //
        // # The two rules that were fighting, and neither module knew
        //
        // `Model::run` pads a batch to its longest row, so a batch of sixteen
        // holding one 512-token se'if and fifteen 20-token ones costs **16×512**
        // through every layer. The standard fix is to sort the work by length —
        // and `girsa_lane::job` says the opposite in as many words: *"in order,
        // because a reader who starts the job and then opens the sefer is at the
        // front of it, and because a job that skipped about would make how far
        // has it got unanswerable."*
        //
        // Both are right, and they are about different things. The job's order
        // is what a reader sees and what makes the run resumable; the batch's
        // order is arithmetic nobody observes. So the grouping happens **here**,
        // inside one call, over rows the job already chose — the job hands them
        // over in reading order, they come back in reading order, and the
        // padding waste in between is gone. That same batch is now one pass of
        // 1×512 and one of 15×20.
        let mut order: Vec<usize> = (0..ids.len()).collect();
        order.sort_by_key(|at| ids.get(*at).map_or(0, Vec::len));

        let mut vectors: Vec<Vec<f32>> = vec![Vec::new(); ids.len()];
        let mut group: Vec<usize> = Vec::new();
        for at in order {
            let longest = group
                .first()
                .and_then(|first| ids.get(*first))
                .map_or(0, Vec::len);
            let mine = ids.get(at).map_or(0, Vec::len);
            // A new pass when this row would more than half again the padding
            // the group is already paying. Cheap to state, and it is the only
            // number here that is a judgement rather than a fact.
            if !group.is_empty() && mine * 2 > longest * 3 {
                self.run_into(&ids, &group, &mut vectors)?;
                group.clear();
            }
            group.push(at);
        }
        self.run_into(&ids, &group, &mut vectors)?;

        Ok(vectors
            .into_iter()
            .zip(truncated)
            .map(|(vector, truncated)| Embedded { vector, truncated })
            .collect())
    }
}

impl Model {
    /// Embed one group of rows and put each answer back where its row was.
    fn run_into(
        &self,
        ids: &[Vec<u32>],
        group: &[usize],
        into: &mut [Vec<f32>],
    ) -> Result<(), ModelError> {
        if group.is_empty() {
            return Ok(());
        }
        let rows: Vec<Vec<u32>> = group
            .iter()
            .filter_map(|at| ids.get(*at).cloned())
            .collect();
        let vectors = self.run(&rows).map_err(ModelError::ran)?;
        for (at, vector) in group.iter().zip(vectors) {
            if let Some(slot) = into.get_mut(*at) {
                *slot = vector;
            }
        }
        Ok(())
    }
}

/// The bits of `config.json` a forward pass needs.
#[derive(Debug, Clone, Deserialize)]
struct Config {
    #[serde(default)]
    model_type: String,
    hidden_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    intermediate_size: usize,
    max_position_embeddings: usize,
    #[serde(default = "two")]
    type_vocab_size: usize,
    vocab_size: usize,
    #[serde(default = "tiny")]
    layer_norm_eps: f64,
    #[serde(default)]
    hidden_act: String,
}

const fn two() -> usize {
    2
}

const fn tiny() -> f64 {
    1e-12
}

impl Config {
    /// Refuse a model this cannot run, before 738 MB is read rather than after.
    fn check(&self, dir: &Path) -> Result<(), ModelError> {
        let refuse = |why: String| {
            Err(ModelError::Unsupported {
                dir: dir.display().to_string(),
                why,
            })
        };
        if !self.model_type.is_empty() && self.model_type != "bert" {
            return refuse(format!(
                "this lane runs BERT encoders and config.json says model_type is \
                 `{}` — a different architecture needs a different forward pass, not a \
                 different setting",
                self.model_type
            ));
        }
        if self.hidden_size == 0 || self.num_attention_heads == 0 {
            return refuse("config.json gives no hidden size or no attention heads".to_string());
        }
        if self.hidden_size % self.num_attention_heads != 0 {
            return refuse(format!(
                "config.json has {} hidden units over {} attention heads, which do not divide",
                self.hidden_size, self.num_attention_heads
            ));
        }
        // gelu and gelu_new differ by which approximation of the same curve;
        // anything else is a different network wearing the same config.
        if !matches!(
            self.hidden_act.as_str(),
            "" | "gelu" | "gelu_new" | "gelu_pytorch_tanh"
        ) {
            return refuse(format!(
                "config.json asks for the {} activation and this lane implements gelu",
                self.hidden_act
            ));
        }
        Ok(())
    }

    /// The same numbers, in the shape `candle-transformers` asks for.
    ///
    /// Read out by hand rather than deserialised straight into
    /// [`BertConfig`], and that is deliberate: that struct requires
    /// `hidden_dropout_prob`, `initializer_range`, `pad_token_id` and
    /// `classifier_dropout` — fields a hand-written `config.json` may not have
    /// and none of which changes a forward pass at inference — so parsing
    /// through it would refuse models this can run. [`Config`] above is what
    /// this crate needs and [`Config::check`] is what it will refuse; this is
    /// only the translation.
    fn as_bert(&self) -> BertConfig {
        BertConfig {
            vocab_size: self.vocab_size,
            hidden_size: self.hidden_size,
            num_hidden_layers: self.num_hidden_layers,
            num_attention_heads: self.num_attention_heads,
            intermediate_size: self.intermediate_size,
            // `check` above has already refused anything that is not one of the
            // gelus, and all three names are the same curve to within an
            // approximation. `HiddenAct::Gelu` is `gelu_erf`, which is what the
            // forward pass this replaced used.
            hidden_act: HiddenAct::Gelu,
            max_position_embeddings: self.max_position_embeddings,
            type_vocab_size: self.type_vocab_size,
            layer_norm_eps: self.layer_norm_eps,
            position_embedding_type: PositionEmbeddingType::Absolute,
            // Inference only. Dropout is identity here and the rest is training
            // metadata; the defaults are named rather than left implicit so a
            // reader can see that none of them reaches the arithmetic.
            hidden_dropout_prob: 0.0,
            initializer_range: 0.02,
            pad_token_id: 0,
            use_cache: false,
            classifier_dropout: None,
            // `None`, not `self.model_type`. `BertModel::load` uses this to
            // retry under a `<model_type>.` prefix when the plain names miss —
            // and `side_loaded` above has *already* resolved the prefix by
            // looking at the tensor names, which is the stronger test. Handing
            // it a model type as well would give it a second, weaker way to
            // guess at a layout this crate has already established.
            model_type: None,
        }
    }
}

/// Which model made a set of vectors.
///
/// Vectors from two different checkpoints are not comparable — the spaces are
/// unrelated — and mixing them would produce a ranked list that looks exactly
/// like a good one. So the store records this and refuses a mismatch
/// ([`crate::vectors`]).
///
/// It is computed from `config.json`, the safetensors **header** (which carries
/// every tensor's name, shape, dtype and offset), the file's length, and 64
/// windows sampled through the weights. That is enough to tell two checkpoints
/// apart and cheap enough to run every time the lane opens. **It is not a
/// security checksum** and is not offered as one: it answers *is this the same
/// model as last time*, not *has anybody tampered with this file*.
fn fingerprint(config: &[u8], weights: &Path) -> Result<String, ModelError> {
    use std::io::{Read, Seek, SeekFrom};

    let mut hash = Fnv::new();
    hash.eat(config);

    let mut file = std::fs::File::open(weights).map_err(ModelError::io(weights))?;
    let length = file.metadata().map_err(ModelError::io(weights))?.len();
    hash.eat(&length.to_le_bytes());

    let mut header_len = [0u8; 8];
    file.read_exact(&mut header_len)
        .map_err(ModelError::io(weights))?;
    let header_len = u64::from_le_bytes(header_len);
    // A safetensors header is tens of kilobytes. Anything claiming more than a
    // megabyte is not one, and reading it would be believing the file about how
    // much memory to allocate.
    if header_len == 0 || header_len > 1 << 20 || header_len + 8 > length {
        return Err(ModelError::bad(
            weights,
            format!("its safetensors header claims to be {header_len} bytes long"),
        ));
    }
    let mut header = vec![0u8; usize::try_from(header_len).unwrap_or(0)];
    file.read_exact(&mut header)
        .map_err(ModelError::io(weights))?;
    hash.eat(&header);

    const WINDOWS: u64 = 64;
    const WINDOW: usize = 4096;
    let body = length.saturating_sub(8 + header_len);
    if body > 0 {
        let step = (body / WINDOWS).max(1);
        let mut window = vec![0u8; WINDOW];
        let mut at = 8 + header_len;
        while at < length {
            file.seek(SeekFrom::Start(at))
                .map_err(ModelError::io(weights))?;
            let want = WINDOW.min(usize::try_from(length - at).unwrap_or(WINDOW));
            let got = &mut window[..want];
            file.read_exact(got).map_err(ModelError::io(weights))?;
            hash.eat(got);
            at = at.saturating_add(step);
        }
    }

    Ok(format!("{:016x}", hash.0))
}

/// FNV-1a, 64-bit. Written out because the alternative is a dependency for
/// eight lines, and because nothing here needs a hash to be cryptographic —
/// see [`fingerprint`], which says so in as many words.
struct Fnv(u64);

impl Fnv {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn eat(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }
}
#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("girsa-lane-model-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        dir
    }

    #[test]
    fn no_model_configured_is_a_sentence_and_not_a_silence() {
        // The whole of "off means off, out loud". A reader who turns the lane
        // on without a model must be told that, not handed an empty list.
        let said = ModelError::NotConfigured.to_string();
        assert!(said.contains("the lane is off"), "{said}");
        assert!(said.contains("downloads nothing"), "{said}");
        assert!(said.contains("config.json"), "{said}");
    }

    #[test]
    fn a_directory_missing_a_file_is_refused_by_the_name_of_the_file() {
        let dir = scratch("half-a-model");
        std::fs::write(dir.join("config.json"), "{}").expect("writes");
        let error = Model::side_loaded(&dir).expect_err("not a model");
        let said = error.to_string();
        assert!(said.contains("tokenizer.json"), "{said}");
        assert!(said.contains("model.safetensors"), "{said}");
        assert!(!said.contains("no config.json"), "{said}");
    }

    #[test]
    fn a_model_that_is_not_a_bert_is_refused_before_the_weights_are_read() {
        let config: Config = serde_json::from_str(
            r#"{"model_type":"llama","hidden_size":768,"num_hidden_layers":12,
                "num_attention_heads":12,"intermediate_size":3072,
                "max_position_embeddings":512,"vocab_size":32000}"#,
        )
        .expect("a config");
        let said = config
            .check(Path::new("somewhere"))
            .expect_err("refused")
            .to_string();
        assert!(said.contains("llama"), "{said}");
        assert!(said.contains("BERT"), "{said}");
    }

    #[test]
    fn heads_that_do_not_divide_the_hidden_size_are_refused_with_both_numbers() {
        let config: Config = serde_json::from_str(
            r#"{"model_type":"bert","hidden_size":768,"num_hidden_layers":12,
                "num_attention_heads":7,"intermediate_size":3072,
                "max_position_embeddings":512,"vocab_size":32000}"#,
        )
        .expect("a config");
        let said = config
            .check(Path::new("somewhere"))
            .expect_err("refused")
            .to_string();
        assert!(said.contains("768") && said.contains('7'), "{said}");
    }

    #[test]
    fn a_fingerprint_changes_when_the_weights_do_and_not_when_the_path_does() {
        let dir = scratch("fingerprint");
        let one = dir.join("one.safetensors");
        let two = dir.join("two.safetensors");
        // A minimal safetensors file: an 8-byte header length, the header, the
        // body. Nothing here parses the body, so the shapes need not be real.
        let write = |path: &Path, body: &[u8]| {
            let header = br#"{"__metadata__":{"a":"b"}}"#;
            let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
            bytes.extend_from_slice(header);
            bytes.extend_from_slice(body);
            std::fs::write(path, bytes).expect("writes");
        };
        write(&one, &[7u8; 100_000]);
        write(&two, &[7u8; 100_000]);
        assert_eq!(
            fingerprint(b"{}", &one).expect("a fingerprint"),
            fingerprint(b"{}", &two).expect("a fingerprint"),
            "the same bytes under a different name are the same model"
        );

        let mut different = vec![7u8; 100_000];
        different[50_000] = 8;
        write(&two, &different);
        assert_ne!(
            fingerprint(b"{}", &one).expect("a fingerprint"),
            fingerprint(b"{}", &two).expect("a fingerprint"),
            "different weights are a different model"
        );

        // And the config counts: the same weights read under a different
        // architecture are not the same thing either.
        assert_ne!(
            fingerprint(b"{}", &one).expect("a fingerprint"),
            fingerprint(br#"{"hidden_size":768}"#, &one).expect("a fingerprint"),
        );
    }

    #[test]
    fn a_file_claiming_an_absurd_header_is_refused_rather_than_believed() {
        let dir = scratch("absurd");
        let path = dir.join("model.safetensors");
        let mut bytes = u64::MAX.to_le_bytes().to_vec();
        bytes.extend_from_slice(b"nowhere near that much");
        std::fs::write(&path, bytes).expect("writes");
        let said = fingerprint(b"{}", &path).expect_err("refused").to_string();
        assert!(said.contains("safetensors header"), "{said}");
    }

    #[test]
    fn how_long_says_the_size_of_the_decision_and_not_a_clock_reading() {
        assert_eq!(how_long(0), None, "nothing left is not a wait");
        assert_eq!(how_long(240).as_deref(), Some("about 54 seconds"));
        assert!(how_long(5_000_545)
            .as_deref()
            .unwrap_or("")
            .ends_with("days"));
        // The measured thirteen. Coarse on purpose: "about 13 days" is the
        // decision a reader is making, and "13 days, 4 hours" is a precision
        // 4.5 segments a second does not have.
        assert_eq!(how_long(5_000_545).as_deref(), Some("about 13 days"));
    }

    /// A whole BERT, small enough to write down.
    ///
    /// Every weight comes from one seeded generator, so the directory this
    /// builds is byte-identical on every machine and the vector it produces is
    /// a number this test can hold.
    fn a_whole_tiny_bert(dir: &Path) {
        const VOCAB: usize = 16;
        const HIDDEN: usize = 8;
        const LAYERS: usize = 2;
        const HEADS: usize = 2;
        const INTER: usize = 16;
        const POSITIONS: usize = 16;
        const TYPES: usize = 2;

        std::fs::create_dir_all(dir).expect("a model directory");
        std::fs::write(
            dir.join("config.json"),
            format!(
                "{{\"model_type\":\"bert\",\"vocab_size\":{VOCAB},\"hidden_size\":{HIDDEN},\
                 \"num_hidden_layers\":{LAYERS},\"num_attention_heads\":{HEADS},\
                 \"intermediate_size\":{INTER},\"max_position_embeddings\":{POSITIONS},\
                 \"type_vocab_size\":{TYPES},\"layer_norm_eps\":1e-12,\"hidden_act\":\"gelu\"}}"
            ),
        )
        .expect("a config");

        // A WordPiece over six tokens and whitespace. Small, and real: it is
        // read by the same `tokenizers` crate a downloaded model's is.
        std::fs::write(
            dir.join("tokenizer.json"),
            r###"{"version":"1.0","truncation":null,"padding":null,
               "added_tokens":[
                 {"id":1,"content":"[CLS]","single_word":false,"lstrip":false,"rstrip":false,"normalized":false,"special":true},
                 {"id":2,"content":"[SEP]","single_word":false,"lstrip":false,"rstrip":false,"normalized":false,"special":true}],
               "normalizer":null,"pre_tokenizer":{"type":"Whitespace"},
               "post_processor":{"type":"TemplateProcessing",
                 "single":[{"SpecialToken":{"id":"[CLS]","type_id":0}},{"Sequence":{"id":"A","type_id":0}},{"SpecialToken":{"id":"[SEP]","type_id":0}}],
                 "pair":[{"Sequence":{"id":"A","type_id":0}}],
                 "special_tokens":{"[CLS]":{"id":"[CLS]","ids":[1],"tokens":["[CLS]"]},"[SEP]":{"id":"[SEP]","ids":[2],"tokens":["[SEP]"]}}},
               "decoder":null,
               "model":{"type":"WordPiece","unk_token":"[UNK]","continuing_subword_prefix":"##","max_input_chars_per_word":100,
                 "vocab":{"[UNK]":0,"[CLS]":1,"[SEP]":2,"[PAD]":3,"א":4,"ב":5,"ג":6,"ד":7}}}"###,
        )
        .expect("a tokenizer");

        // One LCG, so the weights are the same everywhere this runs.
        let mut seed = 0x2026_0806u64;
        #[allow(clippy::cast_precision_loss)]
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let whole = (seed >> 40) as u32;
            (f32::from(u16::try_from(whole & 0xFFFF).unwrap_or(0)) / 65_535.0 - 0.5) * 0.4
        };
        let mut make = |rows: usize, cols: usize| {
            let n = rows * cols;
            let data: Vec<f32> = (0..n).map(|_| next()).collect();
            Tensor::from_vec(data, (rows, cols), &Device::Cpu).expect("a tensor")
        };
        let ones = |n: usize| Tensor::from_vec(vec![1.0f32; n], n, &Device::Cpu).expect("a tensor");
        let zeros =
            |n: usize| Tensor::from_vec(vec![0.0f32; n], n, &Device::Cpu).expect("a tensor");

        let mut tensors: std::collections::HashMap<String, Tensor> =
            std::collections::HashMap::new();
        tensors.insert(
            "embeddings.word_embeddings.weight".into(),
            make(VOCAB, HIDDEN),
        );
        tensors.insert(
            "embeddings.position_embeddings.weight".into(),
            make(POSITIONS, HIDDEN),
        );
        tensors.insert(
            "embeddings.token_type_embeddings.weight".into(),
            make(TYPES, HIDDEN),
        );
        tensors.insert("embeddings.LayerNorm.weight".into(), ones(HIDDEN));
        tensors.insert("embeddings.LayerNorm.bias".into(), zeros(HIDDEN));
        for n in 0..LAYERS {
            let at = format!("encoder.layer.{n}");
            for which in ["query", "key", "value"] {
                tensors.insert(
                    format!("{at}.attention.self.{which}.weight"),
                    make(HIDDEN, HIDDEN),
                );
                tensors.insert(format!("{at}.attention.self.{which}.bias"), zeros(HIDDEN));
            }
            tensors.insert(
                format!("{at}.attention.output.dense.weight"),
                make(HIDDEN, HIDDEN),
            );
            tensors.insert(format!("{at}.attention.output.dense.bias"), zeros(HIDDEN));
            tensors.insert(
                format!("{at}.attention.output.LayerNorm.weight"),
                ones(HIDDEN),
            );
            tensors.insert(
                format!("{at}.attention.output.LayerNorm.bias"),
                zeros(HIDDEN),
            );
            tensors.insert(
                format!("{at}.intermediate.dense.weight"),
                make(INTER, HIDDEN),
            );
            tensors.insert(format!("{at}.intermediate.dense.bias"), zeros(INTER));
            tensors.insert(format!("{at}.output.dense.weight"), make(HIDDEN, INTER));
            tensors.insert(format!("{at}.output.dense.bias"), zeros(HIDDEN));
            tensors.insert(format!("{at}.output.LayerNorm.weight"), ones(HIDDEN));
            tensors.insert(format!("{at}.output.LayerNorm.bias"), zeros(HIDDEN));
        }
        candle_core::safetensors::save(&tensors, dir.join("model.safetensors"))
            .expect("the weights are written");
    }

    #[test]
    fn the_forward_pass_produces_the_vector_it_produced_before() {
        // Nothing tested that this crate computes anything at all. Every other
        // test here is about a refusal — a missing file, a config that will not
        // run, a fingerprint. The one thing the lane exists to do had no check
        // on it, so **swapping the implementation was unverifiable**, which is
        // exactly what the swap to `candle-transformers` needed.
        //
        // The numbers below were produced by the hand-written forward pass this
        // crate carried until 7 August 2026, over a model small enough to write
        // down and deterministic enough to rebuild. They are the fixture, and
        // the tolerance is 1e-5 because a different summation order over the
        // same arithmetic is allowed to move the last bits and nothing else.
        let dir = scratch("tiny-bert");
        a_whole_tiny_bert(&dir);

        let model = Model::side_loaded(&dir).expect("a tiny BERT loads");
        assert_eq!(model.dims(), 8);
        let out = model.embed(&["א ב ג", "ד"]).expect("it embeds");
        assert_eq!(out.len(), 2);

        let unit: f32 = out[0].vector.iter().map(|v| v * v).sum();
        assert!((unit - 1.0).abs() < 1e-5, "not unit length: {unit}");

        const FIRST: [f32; 8] = [
            0.164_169_15,
            0.255_925_5,
            -0.338_898_5,
            0.152_985_92,
            0.127_159_55,
            -0.807_689_2,
            0.247_145_65,
            0.199_202_06,
        ];
        for (at, (got, want)) in out[0].vector.iter().zip(FIRST).enumerate() {
            assert!(
                (got - want).abs() < 1e-5,
                "component {at} moved: {got} against {want}\nwhole vector: {:?}",
                out[0].vector
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_segment_embeds_the_same_alone_as_it_does_in_a_batch() {
        // The property the whole batching rests on, and the one the grouping
        // could have broken. `Model::run` masks the padding out of the attention
        // as well as out of the average precisely so that *"a batch of sixteen
        // does not embed a segment differently from a batch of one"* — which
        // would make a reader's vectors depend on the order the job happened to
        // run in.
        //
        // `embed` now cuts a batch into groups by token length, so that one long
        // se'if does not make fifteen short ones cost 512 tokens each. If the
        // masking were wrong, this is what would say so.
        let dir = scratch("batch-vs-alone");
        a_whole_tiny_bert(&dir);
        let model = Model::side_loaded(&dir).expect("a tiny BERT loads");

        // Deliberately uneven: one long, several short. This is the shape the
        // grouping is for.
        let texts = [
            "א",
            "ב ג ד א ב ג ד א ב ג ד א ב",
            "ג ד",
            "א ב ג",
            "ד א ב ג ד א ב ג ד",
            "ב",
        ];
        let together = model.embed(&texts).expect("the batch embeds");
        assert_eq!(together.len(), texts.len(), "and in the order given");

        for (at, text) in texts.iter().enumerate() {
            let alone = model.embed(&[*text]).expect("one embeds");
            let (a, b) = (&alone[0].vector, &together[at].vector);
            assert_eq!(a.len(), b.len());
            for (n, (one, many)) in a.iter().zip(b).enumerate() {
                assert!(
                    (one - many).abs() < 1e-5,
                    "`{text}` embedded differently in a batch: component {n} is                      {many} against {one} alone"
                );
            }
        }
    }
}
