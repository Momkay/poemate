//! The running session: loaded build plan, route position, current stage. Turns game events into what the overlay shows,
//! and persists the build and the progress per character.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::events::{Event, State};
use crate::plan::{self, Plan};
use crate::pob_model;
use crate::pob_source;
use crate::points;
use crate::rewards;
use crate::route::{Kind, Part, Route};
use crate::route_engine::{convert_cursor, flags_for, Engine, Progress};
use crate::tree;

pub static ROUTE: LazyLock<Route> = LazyLock::new(|| Route::load().expect("bundled route is valid"));
static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(|| Mutex::new(Session::default()));

const BUILD_FILE: &str = "build.txt";
const LEAGUE_FLAG: &str = "LEAGUE_START";
const MAX_PASSIVES_SHOWN: usize = 40;
const NOTES_MAX: usize = 3000;

#[derive(Default)]
struct Session {
    cfg: PathBuf,
    plan: Option<Plan>,
    flags: HashSet<String>,
    cursor: usize,
    /// manual stage choice; cleared when the level moves to another stage by itself
    stage_override: Option<usize>,
    /// the stage the level picks; None = not seen yet, so the first value never clears a saved manual choice
    auto_stage: Option<Option<usize>>,
    character: Option<String>,
    league_start: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    cursor: usize,
    stage_override: Option<usize>,
    /// the route variant `cursor` counts in: the route with the extra side quests has other indexes
    #[serde(default)]
    league_start: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct RouteView {
    pub act: u32,
    pub index: usize,
    pub total: usize,
    pub step: String,
    pub label: String,
    pub hints: Vec<String>,
    pub next: Vec<String>,
    /// gems from this step's quests that the build wants
    pub rewards: Vec<rewards::Offer>,
    /// passive points this step gives (quest hand-ins)
    pub passive_points: u32,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct PassiveView {
    pub name: String,
    pub kind: tree::Kind,
    pub stats: Vec<String>,
    pub known: bool,
    /// the estimated point budget covers it (always true when the level is unknown)
    pub available: bool,
}

/// One node of the mini tree: where it sits, the allocated neighbour it hangs off, and when it is taken.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct TreeDot {
    pub x: f32,
    pub y: f32,
    pub from: Option<(f32, f32)>,
    pub kind: tree::Kind,
    /// "done" (earlier stages), "now" (affordable in this stage), "later" (this stage, not affordable yet)
    pub state: &'static str,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct GemView {
    pub name: String,
    pub support: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct GearView {
    pub slot: String,
    pub name: String,
    pub base: String,
    pub changed: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct PlanView {
    pub stage: usize,
    pub stage_title: String,
    pub stages: Vec<String>,
    pub manual: bool,
    pub auto_order: bool,
    pub class: String,
    pub ascendancy: String,
    pub bandit: String,
    pub passives_total: usize,
    pub passives_available: usize,
    /// estimated points to spend in total: level - 1 + quest rewards passed (None = level unknown)
    pub budget: Option<u32>,
    /// points the earlier stages of the plan already use
    pub spent_before: u32,
    pub passives: Vec<PassiveView>,
    pub tree: Vec<TreeDot>,
    pub masteries: Vec<(String, Vec<String>)>,
    pub new_gems: Vec<String>,
    pub gems: Vec<Vec<GemView>>,
    pub gear: Vec<GearView>,
    /// the build's notes, capped: they travel with every snapshot
    pub notes: String,
}

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Views {
    pub route: Option<RouteView>,
    pub plan: Option<PlanView>,
}

fn safe_name(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

impl Session {
    fn progress_path(&self, character: &str) -> PathBuf {
        self.cfg.join("progress").join(format!("{}.json", safe_name(character)))
    }

    fn save_progress(&self) {
        let Some(c) = &self.character else { return };
        let path = self.progress_path(c);
        let saved = Saved { cursor: self.cursor, stage_override: self.stage_override, league_start: self.league_start };
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
        let tmp = path.with_extension("tmp");
        if serde_json::to_vec(&saved).ok().and_then(|b| std::fs::write(&tmp, b).ok()).is_some() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    fn load_progress(&self, character: &str) -> Option<Saved> {
        serde_json::from_slice(&std::fs::read(self.progress_path(character)).ok()?).ok()
    }

    /// Takes a saved position, converting the cursor when it was saved in the other route variant.
    fn adopt(&mut self, s: Saved) {
        let mut from = self.flags.clone();
        if s.league_start {
            from.insert(LEAGUE_FLAG.into());
        } else {
            from.remove(LEAGUE_FLAG);
        }
        self.cursor = convert_cursor(&ROUTE, s.cursor, &from, &self.flags);
        self.stage_override = s.stage_override;
    }

    fn engine(&self) -> Engine<'static> {
        Engine::new(&ROUTE, &self.flags, Progress { cursor: self.cursor })
    }

    fn on_event(&mut self, ev: &Event, level: u32) {
        match ev {
            Event::AreaChanged { id, .. } => {
                let mut e = self.engine();
                if e.on_area(id) {
                    self.cursor = e.progress.cursor;
                    self.save_progress();
                }
            }
            Event::CharacterDetected { name, .. } => {
                self.character = Some(name.clone());
                match self.load_progress(name) {
                    Some(s) => self.adopt(s),
                    None => self.save_progress(), // first time we see this character: keep the position so far
                }
            }
            Event::LevelChanged { .. } => {}
            Event::Died => {}
        }
        if let Some(plan) = &self.plan {
            let auto = plan.stage_at(level);
            if self.auto_stage.is_some_and(|prev| prev != auto) {
                self.stage_override = None;
            }
            self.auto_stage = Some(auto);
        }
    }

    /// Gems of the current and the next stage, normalized: what is worth picking up now.
    fn wanted_gems(&self, level: u32) -> HashSet<String> {
        let Some(p) = &self.plan else { return HashSet::new() };
        let Some(stage) = self.stage_override.or_else(|| p.stage_at(level)) else { return HashSet::new() };
        p.stages
            .iter()
            .skip(stage)
            .take(2)
            .flat_map(|s| &s.gems)
            .filter(|g| g.enabled)
            .flat_map(|g| g.gems.iter().filter(|x| x.enabled).map(|x| rewards::norm(&x.name)))
            .collect()
    }

    fn views(&self, level: u32) -> Views {
        let e = self.engine();
        let wanted = self.wanted_gems(level);
        let budget = (level > 0).then(|| points::level_points(level) + points::earned(e.done(), &self.flags));
        let class = self.plan.as_ref().map(|p| p.class.as_str()).unwrap_or("");
        let route = e.current().map(|s| RouteView {
            rewards: {
                let ids: Vec<String> = s
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::Frag { kind: Kind::Quest, args } => Some(args.clone()),
                        _ => None,
                    })
                    .flatten()
                    .collect();
                rewards::offers(&ids, class, &wanted)
            },
            passive_points: points::at_step(s, &self.flags),
            act: s.act,
            index: e.progress.cursor,
            total: e.len(),
            step: ROUTE.plain(&s.parts),
            label: s.label.clone(),
            hints: s.hints.iter().map(|h| ROUTE.plain(h)).collect(),
            next: e.window(2).iter().skip(1).map(|s| ROUTE.plain(&s.parts)).collect(),
        });
        let plan = self.plan.as_ref().and_then(|p| {
            let stage = self.stage_override.or_else(|| p.stage_at(level))?;
            let s = p.stages.get(stage)?;
            // ascendancy nodes use the separate ascendancy points, everything else costs one passive point
            let cost = |x: &crate::tree::Step| u32::from(x.ascendancy.is_none());
            let spent_before: u32 = p.stages[..stage].iter().flat_map(|st| &st.passives).map(cost).sum();
            let mut left = budget.map(|b| b.saturating_sub(spent_before));
            let available: Vec<bool> = s
                .passives
                .iter()
                .map(|x| match (&mut left, cost(x)) {
                    (None, _) | (_, 0) => true,
                    (Some(l), c) if *l >= c => {
                        *l -= c;
                        true
                    }
                    _ => false,
                })
                .collect();
            Some(PlanView {
                passives_available: available.iter().filter(|a| **a).count(),
                budget,
                spent_before,
                stage,
                stage_title: s.title.clone(),
                stages: p.stages.iter().map(|s| s.title.clone()).collect(),
                manual: self.stage_override.is_some(),
                auto_order: p.auto_order,
                class: p.class.clone(),
                ascendancy: p.ascendancy.clone(),
                bandit: p.bandit.clone(),
                passives_total: s.passives.len(),
                tree: {
                    // ascendancy nodes sit on their own panel far from the main tree: they would blow up the crop
                    let dot = |x: &crate::tree::Step, state| Some(TreeDot { x: x.pos?.0, y: x.pos?.1, from: x.from, kind: x.kind, state }).filter(|_| x.ascendancy.is_none());
                    let done = p.stages[..stage].iter().flat_map(|st| &st.passives).filter_map(|x| dot(x, "done"));
                    let now = s.passives.iter().zip(&available).filter_map(|(x, a)| dot(x, if *a { "now" } else { "later" }));
                    done.chain(now).collect()
                },
                passives: s
                    .passives
                    .iter()
                    .zip(&available)
                    .take(MAX_PASSIVES_SHOWN)
                    .map(|(x, a)| PassiveView { name: x.name.clone(), kind: x.kind, stats: x.stats.clone(), known: x.known, available: *a })
                    .collect(),
                masteries: s.masteries.clone(),
                new_gems: s.new_gems.clone(),
                gems: s
                    .gems
                    .iter()
                    .filter(|g| g.enabled)
                    .map(|g| g.gems.iter().filter(|x| x.enabled).map(|x| GemView { name: x.name.clone(), support: x.support }).collect())
                    .collect(),
                gear: s
                    .gear
                    .iter()
                    .map(|g| {
                        // PoB calls crafted-from-scratch items "New Item": the base type is the useful part
                        let unnamed = g.item.name == "New Item" && !g.item.base.is_empty();
                        let (name, base) = if unnamed { (g.item.base.clone(), String::new()) } else { (g.item.name.clone(), g.item.base.clone()) };
                        GearView { slot: g.slot.clone(), name, base, changed: g.changed }
                    })
                    .collect(),
                notes: p.notes.chars().take(NOTES_MAX).collect(),
            })
        });
        Views { route, plan }
    }
}

pub fn init(cfg: PathBuf) {
    let mut s = SESSION.lock().unwrap();
    s.league_start = crate::layout::load(&cfg).settings.quests == "all";
    // no build yet (or none at all): the route still needs its flags, or the bandit steps of acts 2 and 10 vanish
    s.flags = flags_for("None", s.league_start);
    s.cfg = cfg;
}

/// Switch between the essential route and the one with every extra side quest; the cursor stays on the same step.
pub fn set_all_quests(on: bool) {
    let mut s = SESSION.lock().unwrap();
    if s.league_start == on {
        return;
    }
    let from = s.flags.clone();
    s.league_start = on;
    if on {
        s.flags.insert(LEAGUE_FLAG.into());
    } else {
        s.flags.remove(LEAGUE_FLAG);
    }
    s.cursor = convert_cursor(&ROUTE, s.cursor, &from, &s.flags);
    s.save_progress();
}

pub fn views(state: &State) -> Views {
    SESSION.lock().unwrap().views(state.level)
}

pub fn on_events(events: &[Event], state: &State) {
    let mut s = SESSION.lock().unwrap();
    for ev in events {
        s.on_event(ev, state.level);
    }
}

/// After the log was read for the first time: place the route at the zone the player is in right now.
pub fn sync_to_current_area(state: &State) {
    let mut s = SESSION.lock().unwrap();
    s.auto_stage = s.plan.as_ref().map(|p| p.stage_at(state.level));
    if let Some(c) = state.character.clone() {
        s.character = Some(c.clone());
        if let Some(saved) = s.load_progress(&c) {
            s.adopt(saved);
            return;
        }
    }
    if !state.area_id.is_empty() {
        let mut e = s.engine();
        if e.on_area(&state.area_id) {
            s.cursor = e.progress.cursor;
        }
    }
}

fn make_plan(text: &str, cfg: &Path, league_start: bool) -> Result<(Plan, HashSet<String>), String> {
    let xml = pob_source::from_text(text).map_err(|e| e.to_string())?;
    let build = pob_model::parse(&xml).map_err(|_| "That isn't a Path of Building export".to_string())?;
    let tree = tree::load(cfg, false, tree::download).map_err(|_| "Couldn't load the passive tree data (offline?)".to_string())?;
    let flags = flags_for(&build.bandit, league_start);
    let mut plan = plan::build(&build, &tree);
    // ids above 65535 are cluster jewel nodes and never in the export; any other unknown id means an old cache
    let unknown = plan.stages.iter().flat_map(|s| &s.passives).any(|p| !p.known && p.id <= 65535);
    if unknown && tree::cache_older_than(cfg, 24 * 3600) {
        if let Ok(fresh) = tree::load(cfg, true, tree::download) {
            plan = plan::build(&build, &fresh);
        }
    }
    Ok((plan, flags))
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub class: String,
    pub ascendancy: String,
    pub stages: usize,
    pub auto_order: bool,
}

/// `keep_stage`: a restored build keeps the manual stage loaded for the character; a new import starts fresh.
fn install(plan: Plan, flags: HashSet<String>, keep_stage: bool) -> ImportSummary {
    let summary = ImportSummary { class: plan.class.clone(), ascendancy: plan.ascendancy.clone(), stages: plan.stages.len(), auto_order: plan.auto_order };
    let mut s = SESSION.lock().unwrap();
    s.plan = Some(plan);
    s.flags = flags;
    if !keep_stage {
        s.stage_override = None;
    }
    s.auto_stage = None;
    summary
}

/// Blocking: may download the build and the tree data.
pub fn import(input: &str) -> Result<ImportSummary, String> {
    let (cfg, league_start) = {
        let s = SESSION.lock().unwrap();
        (s.cfg.clone(), s.league_start)
    };
    let text = pob_source::resolve(input).map_err(|e| e.to_string())?;
    let (plan, flags) = make_plan(&text, &cfg, league_start)?;
    let _ = std::fs::create_dir_all(&cfg);
    let _ = std::fs::write(cfg.join(BUILD_FILE), &text);
    Ok(install(plan, flags, false))
}

/// Blocking: reload the build saved by the last import, if any.
pub fn restore() {
    let (cfg, league_start) = {
        let s = SESSION.lock().unwrap();
        (s.cfg.clone(), s.league_start)
    };
    if let Ok(text) = std::fs::read_to_string(cfg.join(BUILD_FILE)) {
        match make_plan(&text, &cfg, league_start) {
            Ok((plan, flags)) => {
                install(plan, flags, true);
            }
            Err(e) => eprintln!("restoring the saved build failed: {e}"),
        }
    }
}

pub fn step(delta: i32) {
    let mut s = SESSION.lock().unwrap();
    let mut e = s.engine();
    if delta > 0 { e.next() } else { e.prev() }
    s.cursor = e.progress.cursor;
    s.save_progress();
}

pub fn stage(delta: i32, level: u32) {
    let mut s = SESSION.lock().unwrap();
    let Some(plan) = &s.plan else { return };
    let n = plan.stages.len() as i32;
    if n == 0 {
        return;
    }
    let cur = s.stage_override.or_else(|| plan.stage_at(level)).unwrap_or(0) as i32;
    s.stage_override = Some((cur + delta).clamp(0, n - 1) as usize);
    s.save_progress();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with_plan(cfg: &Path) -> Session {
        let text = include_str!("../../fixtures/pob/real1.txt");
        let xml = pob_source::from_text(text).unwrap();
        let build = pob_model::parse(&xml).unwrap();
        let tree = tree::TreeData::from_json(include_str!("../../fixtures/tree/nodes-real1.json")).unwrap();
        Session { cfg: cfg.to_path_buf(), plan: Some(plan::build(&build, &tree)), flags: flags_for(&build.bandit, false), ..Default::default() }
    }

    #[test]
    fn zone_changes_move_the_route_and_level_picks_the_stage() {
        let t = tempfile::tempdir().unwrap();
        let mut s = session_with_plan(t.path());
        let ev = Event::AreaChanged { id: "1_1_town".into(), name: "Lioneye's Watch".into(), level: 1, town: true };
        s.on_event(&ev, 1);
        let v = s.views(1);
        assert_eq!(v.route.as_ref().unwrap().label, "Enemy at the Gate");
        assert_eq!(v.plan.as_ref().unwrap().stage_title, "1) Level 1-12");
        let v = s.views(60);
        assert_eq!(v.plan.unwrap().stage_title, "3) Cruel Lab");
    }

    #[test]
    fn quest_steps_show_the_gems_the_build_wants_from_that_quest() {
        let t = tempfile::tempdir().unwrap();
        let mut s = session_with_plan(t.path());
        s.on_event(&Event::AreaChanged { id: "1_1_town".into(), name: "Lioneye's Watch".into(), level: 1, town: true }, 1);
        let r = s.views(1).route.unwrap();
        assert_eq!(r.label, "Enemy at the Gate"); // "Hand in the quest" step
        assert!(r.rewards.iter().any(|o| o.gem == "Caustic Arrow" && o.npc == "Tarkleigh"), "{:?}", r.rewards);
    }

    #[test]
    fn the_point_budget_limits_what_is_available_now() {
        let t = tempfile::tempdir().unwrap();
        let mut s = session_with_plan(t.path());
        let early = s.views(5).plan.unwrap();
        assert_eq!((early.budget, early.spent_before), (Some(4), 0));
        assert_eq!(early.passives_available, 4);
        assert_eq!(early.passives.iter().filter(|p| p.available).count(), 4);
        assert!(early.passives.iter().take(4).all(|p| p.available) && !early.passives[4].available);
        // a quest hand-in adds a point: Dweller of the Deep is a1q7
        s.cursor = ROUTE.steps.iter().filter(|x| x.applies(&s.flags)).position(|x| x.label == "The Dweller of the Deep").unwrap() + 1;
        assert_eq!(s.views(5).plan.unwrap().budget, Some(5));
        // unknown level: nothing is held back
        let unknown = s.views(0).plan.unwrap();
        assert_eq!(unknown.budget, None);
        assert!(unknown.passives.iter().all(|p| p.available));
        // later stages first need the earlier stages paid for
        let later = s.views(60).plan.unwrap();
        assert!(later.spent_before > 0);
    }

    #[test]
    fn manual_stage_wins_until_the_level_changes_the_stage() {
        let t = tempfile::tempdir().unwrap();
        let mut s = session_with_plan(t.path());
        s.on_event(&Event::LevelChanged { level: 5 }, 5);
        s.stage_override = Some(5);
        s.on_event(&Event::LevelChanged { level: 6 }, 6);
        assert_eq!(s.views(6).plan.unwrap().stage_title, "6) ENDGAME SETUP"); // same auto stage: override stays
        s.on_event(&Event::LevelChanged { level: 33 }, 33);
        let v = s.views(33).plan.unwrap();
        assert_eq!((v.stage_title.as_str(), v.manual), ("2) Normal Lab", false)); // auto stage changed: override cleared
    }

    #[test]
    fn the_route_works_without_any_build() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Session { cfg: t.path().to_path_buf(), flags: flags_for("None", false), ..Default::default() };
        s.on_event(&Event::AreaChanged { id: "1_1_town".into(), name: "Lioneye's Watch".into(), level: 1, town: true }, 1);
        let v = s.views(1);
        assert!(v.plan.is_none());
        let r = v.route.unwrap();
        assert_eq!(r.label, "Enemy at the Gate");
        assert!(r.rewards.is_empty(), "no build, no wanted gems");
        // without flags the bandit decision of act 2 is missing from the route: that is why init sets them
        let with = Engine::new(&ROUTE, &flags_for("None", false), Progress::default()).len();
        let without = Engine::new(&ROUTE, &HashSet::new(), Progress::default()).len();
        assert_ne!(with, without);
    }

    #[test]
    fn a_saved_manual_stage_survives_a_restart_and_the_first_zone_change() {
        let t = tempfile::tempdir().unwrap();
        let mut before = session_with_plan(t.path());
        before.character = Some("Ranger".into());
        before.stage_override = Some(5);
        before.save_progress();

        let mut after = session_with_plan(t.path());
        after.on_event(&Event::CharacterDetected { name: "Ranger".into(), class: "Ranger".into() }, 5);
        after.on_event(&Event::AreaChanged { id: "1_1_town".into(), name: "Lioneye's Watch".into(), level: 5, town: true }, 5);
        assert_eq!(after.stage_override, Some(5));
    }

    #[test]
    fn progress_is_saved_and_restored_per_character() {
        let t = tempfile::tempdir().unwrap();
        let mut s = session_with_plan(t.path());
        s.on_event(&Event::CharacterDetected { name: "Momkay Ranger".into(), class: "Ranger".into() }, 2);
        s.on_event(&Event::AreaChanged { id: "1_1_3".into(), name: "The Mud Flats".into(), level: 3, town: false }, 3);
        let saved_cursor = s.cursor;
        assert!(saved_cursor > 0);

        let mut again = session_with_plan(t.path());
        again.on_event(&Event::CharacterDetected { name: "Momkay Ranger".into(), class: "Ranger".into() }, 2);
        assert_eq!(again.cursor, saved_cursor);
        let mut other = session_with_plan(t.path());
        other.on_event(&Event::CharacterDetected { name: "Other".into(), class: "Witch".into() }, 2);
        assert_eq!(other.cursor, 0);
        std::fs::write(again.progress_path("Momkay Ranger"), "corrupt").unwrap();
        assert!(again.load_progress("Momkay Ranger").is_none());
    }

    #[test]
    fn views_are_empty_without_a_build_and_names_are_file_safe() {
        let s = Session::default();
        assert!(s.views(5).plan.is_none());
        assert_eq!(safe_name("../évil/na me"), "___évil_na_me");
    }
}
