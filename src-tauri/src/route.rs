//! Campaign route: Exile Leveling's route text format (MIT, see docs/CREDITS.md) parsed into typed steps.
//!
//! Format, one step per line:
//!   `➞ {enter|1_1_2} #The Coast`      fragments in braces, `#label` at the end names the area/quest
//!   `    #sub Go {dir|45}`            hint belonging to the step above
//!   `#ifdef FLAG` / `#ifndef FLAG` / `#endif`   conditional blocks (league start, bandit choice, ...)
//!   `#section Act 1`                  start of an act (one per file)

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

const ACTS: [&str; 10] = [
    include_str!("../../data/route/act-1.txt"),
    include_str!("../../data/route/act-2.txt"),
    include_str!("../../data/route/act-3.txt"),
    include_str!("../../data/route/act-4.txt"),
    include_str!("../../data/route/act-5.txt"),
    include_str!("../../data/route/act-6.txt"),
    include_str!("../../data/route/act-7.txt"),
    include_str!("../../data/route/act-8.txt"),
    include_str!("../../data/route/act-9.txt"),
    include_str!("../../data/route/act-10.txt"),
];
const AREAS: &str = include_str!("../../data/areas.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    Enter,
    Kill,
    Quest,
    QuestText,
    Waypoint,
    WaypointGet,
    Dir,
    Logout,
    Trial,
    Portal,
    Crafting,
    Generic,
    Arena,
    Ascend,
    Area,
}

fn kind_of(name: &str) -> Option<Kind> {
    Some(match name {
        "enter" => Kind::Enter,
        "kill" => Kind::Kill,
        "quest" => Kind::Quest,
        "quest_text" => Kind::QuestText,
        "waypoint" => Kind::Waypoint,
        "waypoint_get" => Kind::WaypointGet,
        "dir" => Kind::Dir,
        "logout" => Kind::Logout,
        "trial" => Kind::Trial,
        "portal" => Kind::Portal,
        "crafting" => Kind::Crafting,
        "generic" => Kind::Generic,
        "arena" => Kind::Arena,
        "ascend" => Kind::Ascend,
        "area" => Kind::Area,
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Part {
    Text(String),
    Frag { kind: Kind, args: Vec<String> },
}

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub act: u32,
    pub parts: Vec<Part>,
    /// trailing `#label` (area or quest name)
    pub label: String,
    pub hints: Vec<Vec<Part>>,
    /// all of these must hold for the step to apply: (flag, must be set)
    pub conditions: Vec<(String, bool)>,
}

impl Step {
    /// The zone this step leads into, if it is an "enter" step.
    pub fn enter_area(&self) -> Option<&str> {
        self.parts.iter().find_map(|p| match p {
            Part::Frag { kind: Kind::Enter, args } => args.first().map(String::as_str),
            _ => None,
        })
    }

    pub fn applies(&self, flags: &HashSet<String>) -> bool {
        self.conditions.iter().all(|(f, want)| flags.contains(f) == *want)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RouteError {
    UnknownFragment(String),
    UnclosedBrace(usize),
    UnbalancedIf(usize),
    HintWithoutStep(usize),
}

fn parse_parts(s: &str, line: usize) -> Result<Vec<Part>, RouteError> {
    let mut parts = Vec::new();
    let mut rest = s;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            parts.push(Part::Text(rest[..open].to_string()));
        }
        let close = rest[open..].find('}').ok_or(RouteError::UnclosedBrace(line))? + open;
        let mut it = rest[open + 1..close].split('|');
        let name = it.next().unwrap_or("");
        let kind = kind_of(name).ok_or_else(|| RouteError::UnknownFragment(name.to_string()))?;
        parts.push(Part::Frag { kind, args: it.map(str::to_string).collect() });
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        parts.push(Part::Text(rest.to_string()));
    }
    Ok(parts)
}

pub fn parse_act(text: &str, act: u32) -> Result<Vec<Step>, RouteError> {
    let mut steps: Vec<Step> = Vec::new();
    let mut stack: Vec<(String, bool)> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let n = i + 1;
        if line.is_empty() || line.starts_with("#section") {
            continue;
        }
        if let Some(flag) = line.strip_prefix("#ifdef ") {
            stack.push((flag.trim().to_string(), true));
        } else if let Some(flag) = line.strip_prefix("#ifndef ") {
            stack.push((flag.trim().to_string(), false));
        } else if line == "#endif" {
            stack.pop().ok_or(RouteError::UnbalancedIf(n))?;
        } else if let Some(hint) = line.strip_prefix("#sub") {
            let step = steps.last_mut().ok_or(RouteError::HintWithoutStep(n))?;
            step.hints.push(parse_parts(hint.trim(), n)?);
        } else {
            // a trailing " #label" names the thing; labels never contain braces
            let (body, label) = match line.rfind(" #") {
                Some(p) if !line[p..].contains('{') => (&line[..p], line[p + 2..].trim()),
                _ => (line, ""),
            };
            steps.push(Step { act, parts: parse_parts(body, n)?, label: label.to_string(), hints: Vec::new(), conditions: stack.clone() });
        }
    }
    if !stack.is_empty() {
        return Err(RouteError::UnbalancedIf(text.lines().count()));
    }
    Ok(steps)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Area {
    pub id: String,
    pub name: String,
    pub act: u32,
    pub level: u32,
    pub has_waypoint: bool,
    pub is_town_area: bool,
}

pub struct Route {
    pub steps: Vec<Step>,
    pub areas: HashMap<String, Area>,
}

impl Route {
    pub fn load() -> Result<Self, RouteError> {
        let mut steps = Vec::new();
        for (i, text) in ACTS.iter().enumerate() {
            steps.extend(parse_act(text, i as u32 + 1)?);
        }
        let areas = serde_json::from_str(AREAS).expect("bundled areas.json is valid");
        Ok(Self { steps, areas })
    }

    /// Steps of one act that apply for the given flags (league start, bandit, ...).
    pub fn act_steps<'a>(&'a self, act: u32, flags: &'a HashSet<String>) -> impl Iterator<Item = &'a Step> {
        self.steps.iter().filter(move |s| s.act == act && s.applies(flags))
    }

    /// One-line display text; zones by name, directions as arrows (0 = up, clockwise).
    pub fn plain(&self, parts: &[Part]) -> String {
        const ARROWS: [&str; 8] = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];
        parts
            .iter()
            .map(|p| match p {
                Part::Text(t) => t.clone(),
                Part::Frag { kind, args } => {
                    let a0 = args.first().map(String::as_str).unwrap_or("");
                    match kind {
                        Kind::Enter | Kind::Area | Kind::Waypoint => self.areas.get(a0).map(|a| a.name.clone()).unwrap_or_else(|| a0.to_string()),
                        Kind::Dir => a0.parse::<f32>().ok().map(|d| ARROWS[((d.rem_euclid(360.0) / 45.0).round() as usize) % 8].to_string()).unwrap_or_default(),
                        Kind::WaypointGet => "the waypoint".into(),
                        Kind::Logout => "log out".into(),
                        Kind::Trial => "the Labyrinth trial".into(),
                        Kind::Crafting => "the crafting recipe".into(),
                        Kind::Ascend => "ascend".into(),
                        Kind::Portal => if a0 == "set" { "a portal" } else { "your portal" }.into(),
                        Kind::Quest => "the quest".into(),
                        _ => a0.to_string(),
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(f: &[&str]) -> HashSet<String> {
        f.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn all_ten_acts_parse_and_reference_known_areas() {
        let r = Route::load().unwrap();
        for act in 1..=10 {
            assert!(r.steps.iter().any(|s| s.act == act), "act {act} empty");
        }
        for s in &r.steps {
            for p in s.parts.iter().chain(s.hints.iter().flatten()) {
                if let Part::Frag { kind: Kind::Enter | Kind::Area | Kind::Waypoint, args } = p {
                    if let Some(id) = args.first().filter(|a| a.as_str() != "set") {
                        assert!(r.areas.contains_key(id), "act {} references unknown area {id}", s.act);
                    }
                }
            }
        }
    }

    #[test]
    fn every_enter_step_matches_an_area_the_client_log_can_report() {
        // Client.txt says `Generating level N area "1_1_town"`: our enter ids must be those ids
        let r = Route::load().unwrap();
        let entered: Vec<_> = r.steps.iter().filter_map(Step::enter_area).collect();
        assert!(entered.len() > 100);
        assert!(entered.contains(&"1_1_town") && entered.contains(&"1_1_2"));
    }

    #[test]
    fn act_one_opens_like_the_guide() {
        let r = Route::load().unwrap();
        let none = flags(&[]);
        let first: Vec<_> = r.act_steps(1, &none).take(3).map(|s| r.plain(&s.parts)).collect();
        assert_eq!(first[0], "Find and kill Hillock");
        assert_eq!(first[1], "➞ Lioneye's Watch");
        assert_eq!(first[2], "Hand in the quest");
        let s = r.act_steps(1, &none).nth(2).unwrap();
        assert_eq!(s.label, "Enemy at the Gate");
    }

    #[test]
    fn flags_switch_conditional_blocks() {
        let r = Route::load().unwrap();
        let (off, on) = (flags(&[]), flags(&["LEAGUE_START"]));
        let n_off = r.act_steps(1, &off).count();
        let n_on = r.act_steps(1, &on).count();
        assert_ne!(n_off, n_on);
        // league start only: the Tidal Island detour (Medicine Chest), not in the plain route
        let has = |f: &HashSet<String>| r.act_steps(1, f).any(|s| s.enter_area() == Some("1_1_2a"));
        assert!(has(&on) && !has(&off));
    }

    #[test]
    fn hints_attach_to_their_step_and_render_directions() {
        let r = Route::load().unwrap();
        let none = flags(&[]);
        let s = r.act_steps(2, &none).find(|s| s.enter_area() == Some("1_2_2")).unwrap();
        assert_eq!(s.hints.len(), 1);
        assert_eq!(r.plain(&s.hints[0]), "Go ↗");
        assert_eq!(r.plain(&[Part::Frag { kind: Kind::Dir, args: vec!["270".into()] }, Part::Frag { kind: Kind::Dir, args: vec!["180".into()] }]), "←↓");
    }

    #[test]
    fn malformed_routes_are_errors_not_silent() {
        assert_eq!(parse_act("{bogus|x}", 1).unwrap_err(), RouteError::UnknownFragment("bogus".into()));
        assert_eq!(parse_act("{enter|1_1_2", 1).unwrap_err(), RouteError::UnclosedBrace(1));
        assert_eq!(parse_act("#endif", 1).unwrap_err(), RouteError::UnbalancedIf(1));
        assert_eq!(parse_act("#ifdef A\nstep", 1).unwrap_err(), RouteError::UnbalancedIf(2));
        assert_eq!(parse_act("#sub hint", 1).unwrap_err(), RouteError::HintWithoutStep(1));
    }
}
