//! A tantivy index over the fixture shelf.
//!
//! The `girsa-index build` loop, minus the scans and the personal layer: read
//! each work back off the shelf, ask the touching cache which kinds of link land
//! on each segment, and hand both to [`girsa_search::index::Writer`].
//!
//! The link types are read rather than passed in empty, because the fifth facet
//! (spec.md §9.8) is built from them and an index built without them shows every
//! link facet as zero — which looks exactly like a corpus with no links in it.
//! [`crate::links`] runs first for that reason: `index` implies `links` in the
//! manifest rather than trusting the caller to remember.

use std::path::Path;

use girsa_corpus::import;
use girsa_corpus::work::Work;
use girsa_search::index::SearchIndex;

/// Small: the whole fixture shelf is a few dozen segments, and tantivy's minimum
/// is what decides this rather than the corpus.
const HEAP_BYTES: usize = 15_000_000;

/// Build the index at `index_dir` over the shelf at `root`.
///
/// # Panics
///
/// If the index cannot be built. A test asking the engine a question over an
/// index that silently failed to build gets an empty answer, which is the shape
/// of green this crate exists to end.
pub fn build(root: &Path, index_dir: &Path) {
    std::fs::create_dir_all(index_dir).expect("a fixture index directory");
    let index = SearchIndex::rebuild(index_dir).expect("a fixture index");
    let mut writer = index
        .writer_with_heap(HEAP_BYTES)
        .expect("a fixture index writer");

    let body = std::fs::read_to_string(root.join("works/index.jsonl")).expect("the fixture index");
    let works: Vec<Work> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("a fixture work parses"))
        .collect();

    let mut segments = 0usize;
    for work in &works {
        let imported = import::read_back(root, &work.slug)
            .unwrap_or_else(|e| panic!("{} will not read back: {e}", work.slug));
        let touching = girsa_link::touching::read_back(root, &work.slug).unwrap_or_default();
        let ids: Vec<girsa_corpus::segment::SegmentId> =
            imported.segments.iter().map(|s| s.id.clone()).collect();
        let by_segment = girsa_link::touching::by_segment(&touching, &ids);

        for (at, segment) in imported.segments.iter().enumerate() {
            let kinds: Vec<girsa_link::EdgeType> = by_segment
                .get(at)
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default();
            writer
                .add(segment, &kinds)
                .unwrap_or_else(|e| panic!("cannot index {}: {e}", segment.id));
            segments += 1;
        }
    }
    writer.commit().expect("the fixture index commits");
    assert!(segments > 0, "the fixture index has no segments in it");
}
