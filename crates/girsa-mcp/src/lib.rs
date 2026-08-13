//! The library, answering a program instead of a person.
//!
//! spec.md §12 and BUILDER.md W28: *MCP on both ends.* This is Girsa's end — a
//! Model Context Protocol server over stdio, so an agent can search the corpus,
//! read a segment, resolve a citation, follow the link graph and trace a
//! transmission chain.
//!
//! # It is the same engine, refusals included
//!
//! The tools here call [`girsa_search::bar::Bar`], [`girsa_link::chain`] and
//! [`girsa_app::Shelf`] — the ones the window calls. That is the point. A
//! second, looser query path built for a program would be the first place the
//! guarantees in spec.md §9 stopped holding, and it would stop holding where
//! nobody is watching:
//!
//! - **Torat Emet is the default here too.** `search` runs literally unless the
//!   caller passes `mode`, and the answer says which mode ran. A program cannot
//!   get a widened result by accident any more than a person can.
//! - **A zero result offers the ladder, priced, and applies nothing** (§9.6).
//!   The rungs come back with their counts and the caller has to ask for one.
//! - **A citation with more than one plausible target comes back as a
//!   choice** (rule 6). `resolve` never picks; a caller that wants one picked
//!   has to pick it.
//! - **Every answer carries what was left out** — the chain's [`Refused`], the
//!   caps a walk hit, the works whose incoming links could not be read.
//!
//! [`Refused`]: girsa_link::chain::Refused
//!
//! # Offline, and local
//!
//! stdio only. No socket, no port, nothing bound and nothing dialled: spec.md
//! §14 makes offline the product, and W16's loopback transport for Ksav is
//! token-gated because it is a *socket*. This is a child process reading a pipe,
//! so there is nothing to gate — the program that can talk to it is the program
//! that started it.

pub mod protocol;
pub mod tools;

use std::path::{Path, PathBuf};

use girsa_app::Shelf;
use girsa_corpus::era::Timeline;
use girsa_search::bar::Bar;
use girsa_search::facets::Catalogue;
use girsa_search::index::SearchIndex;

use protocol::{Request, Response};

/// What went wrong before any request could be served.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("cannot open the shelf: {0}")]
    Shelf(String),
    #[error("cannot read {0}: {1}")]
    Catalogue(PathBuf, String),
    #[error("cannot open the search index at {0}: {1}")]
    Index(PathBuf, String),
}

/// The server: a shelf, a search bar, the graph and the time axis.
///
/// Built once at start-up. The index and the catalogue are the expensive part
/// and are held open for the life of the process, the way the window holds
/// them.
pub struct Server {
    root: PathBuf,
    shelf: Shelf,
    bar: Bar,
    timeline: Timeline,
    /// The semantic lane (spec.md §9.9, W30) — off unless the reader turned it
    /// on, and never merged into `search`. It is a **separate tool** here for
    /// exactly the reason it is a separate column in the window: a program that
    /// could get adjacent-by-meaning results out of `search` would have no way
    /// to tell its own caller which kind of answer it had.
    lane: girsa_nearby::Adjacency,
    /// What your own layer holds that the index has not seen (B7, B24).
    ///
    /// Read once at open, like the catalogue: a program asking `search` is entitled
    /// to the same sentence the window's results header shows, and a caller that
    /// cannot complain is exactly who needs it most — a `total` of zero over an
    /// index that has never seen your notes reads to an agent as *this is not in the
    /// library*.
    unindexed: girsa_note::since::Unindexed,
    /// Set once the client has sent `initialize`. A tool call before that is
    /// refused rather than served, because a client that has not handshaken has
    /// not agreed a protocol version and cannot be assumed to read the answer.
    ready: bool,
}

impl Server {
    /// Open the corpus, your layer and the index over both.
    ///
    /// # Errors
    ///
    /// If the shelf, the catalogue or the index cannot be opened. A server that
    /// came up without one of them would answer every search with nothing,
    /// which reads to a caller like an empty library.
    pub fn open(root: &Path, personal: &Path, index: &Path) -> Result<Self, OpenError> {
        let shelf = Shelf::open(root, personal).map_err(|e| OpenError::Shelf(e.to_string()))?;
        let timeline = Timeline::of(root)
            .map_err(|e| OpenError::Catalogue(root.join("works/index.jsonl"), e.to_string()))?;
        let search = SearchIndex::open(index)
            .map_err(|e| OpenError::Index(index.to_path_buf(), e.to_string()))?;
        // With your own tags on it, so `search`'s facets carry a tag column and
        // `narrow_by: "tag"` has somewhere to narrow to (B18).
        let (notes, _) = girsa_note::note::Notes::open(personal);
        let catalogue = Catalogue::of(shelf.works()).tagged(&notes);
        let unindexed = girsa_note::since::Unindexed::of(Some(index), personal);
        // Loads a side-loaded model when the lane is on, which is why it is done
        // once here and not per call. With the lane off — the default — this
        // costs nothing at all.
        let (lane, trouble) = girsa_nearby::Adjacency::open(root, personal, &shelf);
        for line in trouble {
            eprintln!("{line}");
        }
        Ok(Self {
            root: root.to_path_buf(),
            bar: Bar::new(search, catalogue, root),
            shelf,
            timeline,
            lane,
            unindexed,
            ready: false,
        })
    }

    /// What your own layer holds that the index has not seen (B7).
    #[must_use]
    pub fn unindexed(&self) -> girsa_note::since::Unindexed {
        self.unindexed
    }

    /// What it takes to name a place: the shelf, the dates, and a language.
    ///
    /// Hebrew, because a program is not a window and has no language setting —
    /// and it says so here, once, rather than by three call sites each reaching
    /// for `he_title`. The `en_title` reaches a caller as its own field on the
    /// `seforim` answer, which is the right shape for a surface that can read
    /// both.
    #[must_use]
    pub fn names(&self) -> girsa_app::Names<'_> {
        girsa_app::Names::new(
            &self.shelf,
            Some(&self.timeline),
            girsa_app::session::Language::Hebrew,
            girsa_app::CiteStyle::HebrewFull,
        )
    }

    /// The semantic lane, as the `adjacent` tool sees it.
    #[must_use]
    pub fn lane(&self) -> &girsa_nearby::Adjacency {
        &self.lane
    }

    #[must_use]
    pub fn shelf(&self) -> &Shelf {
        &self.shelf
    }

    #[must_use]
    pub fn bar(&self) -> &Bar {
        &self.bar
    }

    #[must_use]
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Serve one line of JSON-RPC.
    ///
    /// `None` for a notification, which by the protocol gets no reply — and a
    /// server that replied to one would break a client that is not waiting for
    /// an answer and would read the reply as the answer to its next request.
    #[must_use]
    pub fn serve(&mut self, line: &str) -> Option<String> {
        let request = match Request::parse(line) {
            Ok(request) => request,
            Err(response) => return Some(response.write()),
        };
        let id = request.id.clone();
        let response = self.dispatch(&request);
        // A notification has no id and gets no reply, whatever it asked for.
        let id = id?;
        Some(response.with_id(id).write())
    }

    fn dispatch(&mut self, request: &Request) -> Response {
        match request.method.as_str() {
            "initialize" => {
                self.ready = true;
                Response::ok(protocol::initialize(request.params.get("protocolVersion")))
            }
            "notifications/initialized" | "notifications/cancelled" | "ping" => {
                Response::ok(serde_json::json!({}))
            }
            "tools/list" => Response::ok(serde_json::json!({ "tools": tools::catalogue() })),
            "tools/call" => {
                if !self.ready {
                    return Response::error(
                        protocol::INVALID_REQUEST,
                        "no `initialize` yet — the protocol version has not been agreed",
                    );
                }
                tools::call(self, &request.params)
            }
            other => Response::error(
                protocol::METHOD_NOT_FOUND,
                &format!("this server does not do `{other}`"),
            ),
        }
    }
}
