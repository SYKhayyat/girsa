//! The semantic lane (spec.md §9.9, W30) — eight commands, and nothing here runs
//! unless the reader turned the lane on.
//!
//! Moved out of `lib.rs` whole (#68): the banner that introduced it there is this
//! file's title, and the text under it is unchanged. Nothing here decides
//! anything — see the note at the top of `lib.rs` for why that rule is not
//! negotiable.

use std::collections::HashMap;
use std::path::PathBuf;

use girsa_app::trouble::{refuse, Code};
use girsa_app::view::{AtRow, CoveredRow, LaneAnswer, LaneProgress, LaneRow, ModelOffer, NearRow};

use crate::{lane_said, Shared, State};

// Eight commands, and the shape of them is the ruling. **Nothing here runs
// unless the reader turned the lane on**: `lane_state` over an off lane opens
// no model and reads no vector, and every other command refuses. The two long
// jobs — bringing a model in and embedding — run on their own thread and emit
// progress, because §9.9 says embedding never blocks reading and a command that
// held the state lock for thirteen days would be a novel way to disagree.

/// A claim on the one long job the lane may be running.
///
/// Held for the life of the thread and released when it drops — however it
/// ends, including a panic. See [`State::working`].
struct Working(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for Working {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Claim the long job, or refuse because one is already running.
///
/// `swap` and not a read-then-write: two callers arriving together must not
/// both read `false`, which is the whole failure this replaces.
fn claim_the_job(shared: &tauri::State<'_, Shared>) -> Result<Working, String> {
    let flag = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        state.working.clone()
    };
    if flag.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(girsa_app::trouble::refuse(
            girsa_app::trouble::Code::AlreadyWorking,
            "the lane is already working — stop it first, or wait for it",
        ));
    }
    Ok(Working(flag))
}

const BRING_EVENT: &str = "lane-bring";
const EMBED_EVENT: &str = "lane-embed";

fn lane_row(state: &State) -> Result<LaneRow, String> {
    let lane = state
        .lane
        .as_ref()
        .ok_or_else(|| state.trouble())?
        .read()
        .map_err(|_| State::poisoned())?;
    let settings = lane.lane().settings();
    let coverage = lane.coverage();
    let lane_state = lane.state();
    Ok(LaneRow {
        state: match &lane_state {
            girsa_lane::State::Off => "off",
            girsa_lane::State::Adrift(_) => "adrift",
            girsa_lane::State::On { .. } => "on",
        },
        said: lane_state.said(),
        coverage: coverage.said(),
        model: settings.model.as_ref().map(|dir| dir.display().to_string()),
        may_fetch: settings.may_fetch,
        everything: lane.lane().chosen().is_everything(),
        chosen: coverage
            .chosen
            .iter()
            .map(|covered| CoveredRow {
                slug: covered.slug.clone(),
                title: covered.title.clone(),
                wanted: covered.wanted,
                embedded: covered.embedded,
            })
            .collect(),
        outside: coverage.outside.len(),
        other_model: coverage.other_model.clone(),
        offer: ModelOffer {
            name: girsa_lane::BEREL.name,
            by: girsa_lane::BEREL.by,
            licence: girsa_lane::BEREL.licence,
            about: girsa_lane::BEREL.about,
            what: girsa_lane::BEREL.what,
            bytes: girsa_lane::BEREL.bytes,
        },
    })
}

#[tauri::command(async)]
pub(crate) fn lane_state(shared: tauri::State<'_, Shared>) -> Result<LaneRow, String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    lane_row(&state)
}

/// Ask the lane. Never an error — a lane that is off, adrift or empty comes
/// back with `refused` set and the coverage sentence said.
#[tauri::command(async)]
pub(crate) fn lane_ask(
    shared: tauri::State<'_, Shared>,
    text: String,
    limit: Option<usize>,
) -> Result<LaneAnswer, String> {
    // **The lock is taken to read the handles and dropped before the ask.**
    // `Adjacency::ask` embeds the query with a neural model and searches every
    // vector store in the lane; it is the heaviest single operation in the
    // application, and it ran under the global guard for all of it. `ask` is
    // `&self` and the lane holds nothing of ours, so the only reason it was
    // ever under the guard is that the guard was around everything — which is
    // what `find` says above, in the same words, about a cheaper command.
    let (shelf, lane, timeline, scoped, language, cite) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf.as_ref().ok_or_else(|| state.trouble())?.clone();
        let lane = state.lane.as_ref().ok_or_else(|| state.trouble())?.clone();
        (
            shelf,
            lane,
            state.timeline.clone(),
            // Scoped by the same chip the literal search is scoped by, so *the
            // whole shelf* and *this sefer* mean the same thing in both columns.
            state
                .chips
                .scope
                .works()
                .into_iter()
                .collect::<Vec<String>>(),
            state.session.language,
            state.session.cite,
        )
    };
    // Shelf, then lane. See the note on `State::lane`: the settings commands
    // take the pair in that order and one order is the whole of not deadlocking.
    let shelf = shelf.read().map_err(|_| State::poisoned())?;
    let lane = lane.read().map_err(|_| State::poisoned())?;
    // The same `Names` the search column uses, so the two columns beside each
    // other cannot call one sefer by two names.
    let names = girsa_app::Names::new(&shelf, timeline.as_ref(), language, cite);
    let answer = lane.ask(&names, &text, &scoped, limit.unwrap_or(girsa_lane::MOST));
    Ok(LaneAnswer {
        label: answer.label,
        measured: answer.measured,
        near: answer
            .near
            .iter()
            .map(|near| NearRow {
                at: AtRow::of(&near.at),
                text: near.text.clone(),
                nearness: near.nearness,
            })
            .collect(),
        coverage: answer.coverage,
        refused: answer.refused,
        asking: answer.asking,
        shortlisted: answer.shortlisted,
        unresolved: answer.unresolved,
    })
}

/// Turn the lane on or off, and point it at a model.
///
/// Turning it on loads the model, which is hundreds of megabytes — so this can
/// take a moment, and a model that will not load is **not** an error here. It is
/// [`girsa_lane::State::Adrift`], which the header says out loud rather than a
/// click that failed silently.
#[tauri::command(async)]
pub(crate) fn lane_set(
    shared: tauri::State<'_, Shared>,
    on: bool,
    model: Option<String>,
) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
    let was = lane.lane().settings().clone();
    let settings = girsa_lane::Settings {
        on,
        model: model.map(PathBuf::from).or(was.model),
        may_fetch: was.may_fetch,
    };
    let done = lane.set(settings, &shelf).map_err(lane_said);
    // Both guards, in the reverse of the order they were taken. `lane_row`
    // reads the state and would otherwise be borrowing it while these are alive.
    drop(lane);
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Let Girsa go and get a model, or stop it being able to.
///
/// Its own command rather than a field on `lane_set`, because it is its own
/// decision: spec.md §14 says Girsa never *needs* the network, and this is the
/// switch that makes that sentence true in a fresh install.
#[tauri::command(async)]
pub(crate) fn lane_allow_fetch(
    shared: tauri::State<'_, Shared>,
    allow: bool,
) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
    let settings = girsa_lane::Settings {
        may_fetch: allow,
        ..lane.lane().settings().clone()
    };
    let done = lane.set(settings, &shelf).map_err(lane_said);
    // Both guards, in the reverse of the order they were taken. `lane_row`
    // reads the state and would otherwise be borrowing it while these are alive.
    drop(lane);
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Bring a model in. Needs `lane_allow_fetch` first.
///
/// Runs on its own thread and emits [`BRING_EVENT`], so the panel draws a bar
/// and the reader can carry on learning. Stopping is closing the panel: the
/// `.part` file stays and the next press resumes where it left off.
#[tauri::command(async)]
pub(crate) fn lane_bring(
    app: tauri::AppHandle,
    shared: tauri::State<'_, Shared>,
) -> Result<(), String> {
    use tauri::Emitter;
    let (personal, may_fetch) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let lane = state
            .lane
            .as_ref()
            .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
            .read()
            .map_err(|_| State::poisoned())?;
        (
            lane.lane().personal().to_path_buf(),
            lane.lane().settings().may_fetch,
        )
    };
    if !may_fetch {
        return Err(girsa_lane::BringError::NotAllowed.to_string());
    }
    // The same claim, and the same reason: two clicks used to start two
    // downloads of a multi-hundred-megabyte model into one directory.
    let working = claim_the_job(&shared)?;
    std::thread::spawn(move || {
        let _working = working;
        let mut last = 0u64;
        let brought = girsa_lane::bring(&personal, true, &mut |progress| {
            let mb = progress.bytes / 1_048_576;
            if mb != last {
                last = mb;
                let _ = app.emit(
                    BRING_EVENT,
                    LaneProgress {
                        doing: "bring",
                        what: progress.file.clone(),
                        done: progress.bytes,
                        of: progress.want.unwrap_or(0),
                        trouble: None,
                    },
                );
            }
            true
        });
        let trouble = match &brought {
            Ok(dir) => {
                // Point the lane at what just landed, and turn it on. The reader
                // pressed a button that says *bring it in*; making them then find
                // the directory they never chose would be a joke.
                if let Ok(mut state) = tauri::Manager::state::<Shared>(&app).lock() {
                    let State { shelf, lane, .. } = &mut *state;
                    // Shelf, then lane. The one order — see `State::lane`.
                    let shelf = shelf.as_ref().and_then(|s| s.read().ok());
                    let lane = lane.as_ref().and_then(|l| l.write().ok());
                    if let (Some(shelf), Some(mut lane)) = (shelf, lane) {
                        let settings = girsa_lane::Settings {
                            on: true,
                            model: Some(dir.clone()),
                            may_fetch: true,
                        };
                        if let Err(e) = lane.set(settings, &shelf) {
                            eprintln!("the model came in but the setting would not save: {e}");
                        }
                    }
                }
                None
            }
            Err(e) => Some(e.to_string()),
        };
        let _ = app.emit(
            BRING_EVENT,
            LaneProgress {
                doing: "done",
                what: girsa_lane::BEREL.name.to_string(),
                done: 0,
                of: 0,
                trouble,
            },
        );
    });
    Ok(())
}

/// Put a sefer in the lane, or take it out. `all` chooses the whole library.
#[tauri::command(async)]
pub(crate) fn lane_choose(
    shared: tauri::State<'_, Shared>,
    slug: Option<String>,
    add: bool,
    all: bool,
) -> Result<LaneRow, String> {
    let mut state = shared.lock().map_err(|_| State::poisoned())?;
    let no_shelf = state.trouble();
    // Two disjoint fields: the shelf read, the lane written. Taken apart by
    // hand because a method on `State` would borrow the whole of it.
    let State { shelf, lane, .. } = &mut *state;
    let shelf = shelf
        .as_ref()
        .ok_or(no_shelf)?
        .read()
        .map_err(|_| State::poisoned())?;
    // Shelf above, lane here: the one order. See `State::lane`.
    let mut lane = lane
        .as_ref()
        .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
        .write()
        .map_err(|_| State::poisoned())?;
    let mut chosen = lane.lane().chosen().clone();
    if all {
        chosen = if add {
            girsa_lane::Chosen::everything()
        } else {
            girsa_lane::Chosen::nothing()
        };
    } else if let Some(slug) = slug {
        if add {
            chosen = chosen.with_work(&slug);
        } else if chosen.is_everything() {
            return Err(
                "the whole library is in the lane — turn that off first, then choose seforim"
                    .to_string(),
            );
        } else if !chosen.without_work(&slug) {
            return Err(girsa_app::trouble::refuse(
                girsa_app::trouble::Code::NoSuch,
                format!("{slug} was not in the lane"),
            ));
        }
    }
    let done = lane.choose(chosen, &shelf).map_err(lane_said);
    drop(lane);
    drop(shelf);
    done?;
    lane_row(&state)
}

/// Embed what is chosen, on its own thread, emitting [`EMBED_EVENT`].
///
/// The lane is **cloned** for the thread, which shares the one loaded model
/// rather than loading a second — see `girsa_lane::Lane`. The state lock is held
/// only long enough to take that clone, so nothing about reading a sefer waits
/// on this.
#[tauri::command(async)]
pub(crate) fn lane_embed(
    app: tauri::AppHandle,
    shared: tauri::State<'_, Shared>,
) -> Result<(), String> {
    use tauri::Emitter;
    let (lane, root, slugs, titles, stop) = {
        let state = shared.lock().map_err(|_| State::poisoned())?;
        let shelf = state.shelf()?;
        let held = state
            .lane
            .as_ref()
            .ok_or_else(|| refuse(Code::NoLane, "there is no lane here"))?
            .read()
            .map_err(|_| State::poisoned())?;
        if !held.state().is_on() {
            return Err(girsa_app::trouble::refuse(
                Code::NoLane,
                "the semantic lane is off",
            ));
        }
        let slugs = girsa_nearby::adjacent::in_the_lane(&shelf, held.lane().chosen());
        let titles: HashMap<String, String> = slugs
            .iter()
            .map(|slug| {
                (
                    slug.clone(),
                    shelf
                        .work(slug)
                        .map_or_else(|| slug.clone(), |work| work.he_title.clone()),
                )
            })
            .collect();
        (
            held.for_thread(),
            shelf.root().to_path_buf(),
            slugs,
            titles,
            state.stop_embedding.clone(),
        )
    };
    // **One job at a time, claimed here.** See `State::working` for what a
    // second one did. The claim comes before `stop.store(false)` and not after:
    // clearing the stop flag is the thing a second run must never get to do.
    let working = claim_the_job(&shared)?;
    stop.store(false, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || {
        // Released however this thread ends — the loop finishing, a `break` on
        // Stop, or a panic inside the model. A flag that is only cleared on the
        // happy path is a lane that can never be embedded again without a
        // restart, which is a worse failure than the one it prevents.
        let _working = working;
        let mut trouble: Vec<String> = Vec::new();
        'seforim: for slug in slugs {
            let title = titles.get(&slug).cloned().unwrap_or_else(|| slug.clone());
            let mut run = match lane.run(&root, &slug) {
                Ok(run) => run,
                Err(e) => {
                    trouble.push(format!("{title}: {e}"));
                    continue;
                }
            };
            trouble.extend(run.trouble().iter().cloned());
            if let Some(made_by) = run.made_by_something_else() {
                trouble.push(format!(
                    "{title}: its vectors were made by {made_by} and are not being added to"
                ));
                continue;
            }
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break 'seforim;
                }
                match run.step() {
                    Ok(0) => break,
                    Ok(_) => {
                        let _ = app.emit(
                            EMBED_EVENT,
                            LaneProgress {
                                doing: "embed",
                                what: title.clone(),
                                done: run.job().done() as u64,
                                of: run.job().wanted() as u64,
                                trouble: None,
                            },
                        );
                    }
                    Err(e) => {
                        trouble.push(format!("{title}: {e}"));
                        break;
                    }
                }
            }
        }
        // The coverage sentence is recomputed once, at the end, off the disk —
        // never cached and never guessed at from the counters above.
        if let Ok(mut state) = tauri::Manager::state::<Shared>(&app).lock() {
            let State { shelf, lane, .. } = &mut *state;
            // Shelf, then lane. The one order — see `State::lane`.
            let shelf = shelf.as_ref().and_then(|s| s.read().ok());
            let held = lane.as_ref().and_then(|l| l.write().ok());
            if let (Some(shelf), Some(mut held)) = (shelf, held) {
                held.refresh(&shelf);
            }
        }
        let _ = app.emit(
            EMBED_EVENT,
            LaneProgress {
                doing: "done",
                what: String::new(),
                done: 0,
                of: 0,
                trouble: (!trouble.is_empty()).then(|| trouble.join("\n")),
            },
        );
    });
    Ok(())
}

/// Stop the embedding job. Costs the batch it is on and nothing else.
#[tauri::command(async)]
pub(crate) fn lane_stop(shared: tauri::State<'_, Shared>) -> Result<(), String> {
    let state = shared.lock().map_err(|_| State::poisoned())?;
    state
        .stop_embedding
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
