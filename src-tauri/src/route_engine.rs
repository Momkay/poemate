//! Walks the player through the route. Zone changes from Client.txt advance it, manual next/prev always wins.
//!
//! Rule: when the player enters zone X, the cursor jumps to the next "enter X" step at or after the
//! cursor, and a few steps ahead are tried first so skipped steps don't strand the player. Going
//! backwards (e.g. back to town) never moves the cursor back: the town enter steps of a later part
//! of the act are only matched when they lie ahead.

use std::collections::HashSet;

use serde::Serialize;

use crate::route::{Route, Step};

/// How far past the cursor a zone change may jump (steps skipped by the player).
const LOOKAHEAD: usize = 40;
/// Towns are entered all the time (portal to sell), so only a close enter step counts as progress.
const TOWN_LOOKAHEAD: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// index into the filtered step list
    pub cursor: usize,
}

pub struct Engine<'a> {
    route: &'a Route,
    steps: Vec<&'a Step>,
    pub progress: Progress,
}

impl<'a> Engine<'a> {
    pub fn new(route: &'a Route, flags: &HashSet<String>, progress: Progress) -> Self {
        let steps: Vec<&Step> = route.steps.iter().filter(|s| s.applies(flags)).collect();
        let cursor = progress.cursor.min(steps.len().saturating_sub(1));
        Self { route, steps, progress: Progress { cursor } }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// (step number, number of steps) within the act of the current step; the whole route is too long to mean anything.
    pub fn act_progress(&self) -> (usize, usize) {
        let Some(cur) = self.current() else { return (0, 0) };
        let first = self.steps.iter().position(|s| s.act == cur.act).unwrap_or(0);
        (self.progress.cursor - first + 1, self.steps.iter().filter(|s| s.act == cur.act).count())
    }

    pub fn current(&self) -> Option<&'a Step> {
        self.steps.get(self.progress.cursor).copied()
    }

    /// Steps behind the cursor.
    pub fn done(&self) -> &[&'a Step] {
        &self.steps[..self.progress.cursor.min(self.steps.len())]
    }

    /// Current step and the next `n` ones.
    pub fn window(&self, n: usize) -> &[&'a Step] {
        let c = self.progress.cursor.min(self.steps.len());
        &self.steps[c..(c + 1 + n).min(self.steps.len())]
    }

    pub fn next(&mut self) {
        if self.progress.cursor + 1 < self.steps.len() {
            self.progress.cursor += 1;
        }
    }

    pub fn prev(&mut self) {
        self.progress.cursor = self.progress.cursor.saturating_sub(1);
    }

    /// The player entered `area_id`. Returns true when the cursor moved.
    pub fn on_area(&mut self, area_id: &str) -> bool {
        let from = self.progress.cursor;
        let town = self.route.areas.get(area_id).is_some_and(|a| a.is_town_area);
        let end = (from + if town { TOWN_LOOKAHEAD } else { LOOKAHEAD } + 1).min(self.steps.len());
        let hit = self.steps[from..end].iter().position(|s| s.enter_area() == Some(area_id));
        match hit {
            // landing on the zone's enter step means that step is done: show what comes after it
            Some(i) => {
                self.progress.cursor = (from + i + 1).min(self.steps.len().saturating_sub(1));
                true
            }
            None => false,
        }
    }

    /// Act of the step the cursor is on.
    #[cfg(test)]
    pub fn act(&self) -> Option<u32> {
        self.current().map(|s| s.act)
    }
}

/// Where a cursor lands when the same route is walked with other flags (the step it was on, or the next one that still applies).
pub fn convert_cursor(route: &Route, cursor: usize, from: &HashSet<String>, to: &HashSet<String>) -> usize {
    let Some(at) = route.steps.iter().enumerate().filter(|(_, s)| s.applies(from)).nth(cursor).map(|(i, _)| i) else { return 0 };
    route.steps.iter().enumerate().filter(|(_, s)| s.applies(to)).position(|(i, _)| i >= at).unwrap_or(0)
}

/// Route flags from the build and the user's playstyle.
pub fn flags_for(bandit: &str, league_start: bool) -> HashSet<String> {
    let mut f = HashSet::new();
    if league_start {
        f.insert("LEAGUE_START".to_string());
    }
    match bandit.to_lowercase().as_str() {
        "alira" => f.insert("BANDIT_ALIRA".into()),
        "oak" => f.insert("BANDIT_OAK".into()),
        "kraityn" => f.insert("BANDIT_KRAITYN".into()),
        _ => f.insert("BANDIT_KILL".into()), // PoB says "None" for killing all three
    };
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route() -> Route {
        Route::load().unwrap()
    }

    #[test]
    fn zone_changes_advance_and_replaying_a_visited_zone_does_not_go_back() {
        let r = route();
        let flags = flags_for("Alira", false);
        let mut e = Engine::new(&r, &flags, Progress::default());
        assert_eq!(r.plain(&e.current().unwrap().parts), "Find and kill Hillock");

        assert!(e.on_area("1_1_town")); // killed Hillock, entered Lioneye's Watch
        assert_eq!(e.current().unwrap().label, "Enemy at the Gate"); // "Hand in the quest" is next
        let after_town = e.progress.cursor;

        assert!(e.on_area("1_1_2")); // The Coast
        assert!(e.progress.cursor > after_town);
        let on_coast = e.progress.cursor;
        assert!(!e.on_area("1_1_town") || e.progress.cursor >= on_coast, "cursor never moves back");
    }

    #[test]
    fn portaling_to_town_mid_zone_does_not_skip_the_route() {
        let r = route();
        let flags = flags_for("Alira", false);
        let mut e = Engine::new(&r, &flags, Progress::default());
        e.on_area("1_1_town");
        e.on_area("1_1_2");
        e.on_area("1_1_3"); // Mud Flats; the next town enter step is further than TOWN_LOOKAHEAD away
        let at = e.progress.cursor;
        assert!(!e.on_area("1_1_town"));
        assert_eq!(e.progress.cursor, at);
    }

    #[test]
    fn skipping_ahead_works_and_unknown_or_far_zones_are_ignored() {
        let r = route();
        let flags = flags_for("Alira", false);
        let mut e = Engine::new(&r, &flags, Progress::default());
        assert!(e.on_area("1_1_3")); // Mud Flats without visiting the Coast step by step
        assert!(!e.on_area("not_a_zone"));
        let at = e.progress.cursor;
        assert!(!e.on_area("2_10_town")); // act 10 is far beyond the lookahead
        assert_eq!(e.progress.cursor, at);
    }

    #[test]
    fn manual_next_prev_are_clamped_and_progress_restores() {
        let r = route();
        let flags = flags_for("Oak", true);
        let mut e = Engine::new(&r, &flags, Progress::default());
        e.prev();
        assert_eq!(e.progress.cursor, 0);
        e.next();
        e.next();
        let saved = e.progress.clone();
        let e2 = Engine::new(&r, &flags, saved.clone());
        assert_eq!(e2.progress, saved);
        let huge = Engine::new(&r, &flags, Progress { cursor: 1_000_000 });
        assert_eq!(huge.progress.cursor, huge.len() - 1);
        assert_eq!(e2.window(2).len(), 3);
    }

    #[test]
    fn the_whole_campaign_can_be_walked_by_zone_changes_alone() {
        // feed every "enter" in route order, like a player following the guide: the cursor must only ever move forward and end in act 10
        let r = route();
        let flags = flags_for("Alira", false);
        let mut e = Engine::new(&r, &flags, Progress::default());
        let enters: Vec<String> = r.steps.iter().filter(|s| s.applies(&flags)).filter_map(|s| s.enter_area().map(str::to_string)).collect();
        let mut last = 0;
        for id in &enters {
            e.on_area(id);
            assert!(e.progress.cursor >= last);
            last = e.progress.cursor;
        }
        assert_eq!(e.act(), Some(10));
        assert!(e.progress.cursor + 5 >= e.len(), "ended at {}/{}", e.progress.cursor, e.len());
    }

    #[test]
    fn bandit_flags_follow_the_build() {
        assert!(flags_for("Alira", false).contains("BANDIT_ALIRA"));
        assert!(flags_for("Kraityn", true).contains("LEAGUE_START"));
        assert!(flags_for("None", false).contains("BANDIT_KILL"));
    }

    #[test]
    fn the_everything_route_adds_side_quests_but_never_changes_the_passive_points() {
        let r = route();
        let (lean, full) = (flags_for("None", false), flags_for("None", true));
        let (l, f) = (Engine::new(&r, &lean, Progress::default()), Engine::new(&r, &full, Progress::default()));
        assert!(l.len() < f.len(), "{} vs {}", l.len(), f.len());
        let end = Progress { cursor: usize::MAX };
        let total = |fl: &HashSet<String>| crate::points::earned(Engine::new(&r, fl, end.clone()).done(), fl);
        assert_eq!(total(&lean), total(&full));
    }

    #[test]
    fn the_cursor_stays_on_its_step_when_the_route_variant_changes() {
        let r = route();
        let (lean, full) = (flags_for("None", false), flags_for("None", true));
        let steps_lean: Vec<_> = r.steps.iter().filter(|s| s.applies(&lean)).collect();
        let steps_full: Vec<_> = r.steps.iter().filter(|s| s.applies(&full)).collect();
        for c in 0..steps_lean.len() {
            let to = convert_cursor(&r, c, &lean, &full);
            if steps_lean[c].applies(&full) {
                assert!(std::ptr::eq(steps_full[to], steps_lean[c]), "lean step {c} moved");
            }
            assert!(steps_full[to].act >= steps_lean[c].act, "lean step {c} went back an act");
        }
        for c in 0..steps_full.len() {
            let to = convert_cursor(&r, c, &full, &lean);
            assert!(steps_lean[to].act >= steps_full[c].act, "full step {c} went back an act");
        }
    }

    #[test]
    fn progress_counts_inside_the_act() {
        let r = route();
        let flags = flags_for("None", false);
        let steps: Vec<_> = r.steps.iter().filter(|s| s.applies(&flags)).collect();
        for act in 1..=10 {
            let idx: Vec<usize> = steps.iter().enumerate().filter(|(_, s)| s.act == act).map(|(i, _)| i).collect();
            let at = |c: usize| Engine::new(&r, &flags, Progress { cursor: c }).act_progress();
            assert_eq!(at(idx[0]), (1, idx.len()), "first step of act {act}");
            assert_eq!(at(*idx.last().unwrap()), (idx.len(), idx.len()), "last step of act {act}");
        }
    }
}
