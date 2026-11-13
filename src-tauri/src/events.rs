//! Turns `Client.txt` lines into game events. Only system lines are parsed; chat is never stored.
//!
//! A system line is `<timestamp> [INFO Client <pid>] <body>`. Chat bodies start with a channel
//! sigil (`@`, `#`, `%`, `&`, `$`) and the player's text can't impersonate a system line,
//! because every body we accept is matched from its very first character.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    AreaChanged { id: String, name: String, level: u32, town: bool },
    LevelChanged { level: u32 },
    CharacterDetected { name: String, class: String },
    Died,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct State {
    pub area_id: String,
    pub area_name: String,
    pub area_level: u32,
    pub town: bool,
    pub character: Option<String>,
    pub class: Option<String>,
    pub level: u32,
    pub deaths: u32,
}

#[derive(Default)]
pub struct Parser {
    pub state: State,
    pending_area: Option<(String, u32)>,
}

fn body(line: &str) -> Option<&str> {
    let (head, rest) = line.split_once("] ")?;
    // the real log writes the area line at DEBUG level and the rest at INFO
    (head.contains("[INFO Client ") || head.contains("[DEBUG Client ")).then_some(rest)
}

fn is_town(id: &str) -> bool {
    id.ends_with("_town") || id.contains("Hideout")
}

impl Parser {
    pub fn feed(&mut self, line: &str) -> Vec<Event> {
        let Some(b) = body(line) else { return Vec::new() };

        if let Some(rest) = b.strip_prefix("Generating level ") {
            if let Some((lvl, rest)) = rest.split_once(" area \"") {
                if let (Ok(level), Some((id, _))) = (lvl.parse(), rest.split_once('"')) {
                    self.pending_area = Some((id.to_string(), level));
                }
            }
            return Vec::new();
        }

        let Some(b) = b.strip_prefix(": ") else { return Vec::new() };

        if let Some(name) = b.strip_prefix("You have entered ").and_then(|n| n.strip_suffix('.')) {
            let (id, level) = self.pending_area.take().unwrap_or_default();
            let town = is_town(&id);
            let s = &mut self.state;
            (s.area_id, s.area_name, s.area_level, s.town) = (id.clone(), name.to_string(), level, town);
            return vec![Event::AreaChanged { id, name: name.to_string(), level, town }];
        }

        if let Some((who, lvl)) = b.split_once(" is now level ") {
            let (Ok(level), Some((name, class))) = (lvl.parse::<u32>(), who.rsplit_once(" (")) else {
                return Vec::new();
            };
            let Some(class) = class.strip_suffix(')') else { return Vec::new() };
            let mut ev = Vec::new();
            let s = &mut self.state;
            if s.character.as_deref() != Some(name) {
                s.character = Some(name.to_string());
                s.deaths = 0;
                ev.push(Event::CharacterDetected { name: name.to_string(), class: class.to_string() });
            }
            s.class = Some(class.to_string());
            if s.level != level {
                s.level = level;
                ev.push(Event::LevelChanged { level });
            }
            return ev;
        }

        // other players' deaths (party) must not count, and we only know "ours" after a level-up line
        if let Some(name) = b.strip_suffix(" has been slain.") {
            if self.state.character.as_deref() == Some(name) {
                self.state.deaths += 1;
                return vec![Event::Died];
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = include_str!("../../fixtures/client/session.txt");

    fn run() -> (Parser, Vec<Event>) {
        let mut p = Parser::default();
        let ev = SESSION.lines().flat_map(|l| p.feed(l)).collect();
        (p, ev)
    }

    fn area(id: &str, name: &str, level: u32, town: bool) -> Event {
        Event::AreaChanged { id: id.into(), name: name.into(), level, town }
    }

    #[test]
    fn session_produces_exactly_the_expected_events() {
        let (_, ev) = run();
        assert_eq!(
            ev,
            vec![
                area("1_1_town", "Lioneye's Watch", 1, true),
                area("1_1_1", "The Twilight Strand", 1, false),
                Event::CharacterDetected { name: "Mateusz".into(), class: "Marauder".into() },
                Event::LevelChanged { level: 2 },
                Event::Died,
                area("1_1_town", "Lioneye's Watch", 1, true),
                area("1_1_2", "The Coast", 2, false),
                Event::LevelChanged { level: 3 },
                area("HideoutShaped", "Shaped Hideout", 68, true),
                Event::LevelChanged { level: 4 }, // ascendancy changes the class name, same character
                Event::CharacterDetected { name: "OtherChar".into(), class: "Witch".into() },
                Event::LevelChanged { level: 5 },
            ]
        );
    }

    #[test]
    fn chat_cannot_spoof_system_lines() {
        let (p, ev) = run();
        // spoofed whisper/global/party lines in the fixture must not leak in
        assert!(!ev.iter().any(|e| matches!(e, Event::AreaChanged { name, .. } if name == "Oriath")));
        assert!(!ev.iter().any(|e| matches!(e, Event::CharacterDetected { name, .. } if name == "Fake")));
        assert_ne!(p.state.area_level, 99);
    }

    #[test]
    fn state_reflects_the_last_events_and_counts_only_our_deaths() {
        let (p, _) = run();
        assert_eq!(p.state.character.as_deref(), Some("OtherChar"));
        assert_eq!(p.state.level, 5);
        assert_eq!(p.state.area_name, "Shaped Hideout");
        // Mateusz died once, then OtherChar took over (deaths reset)
        assert_eq!(p.state.deaths, 0);
    }

    #[test]
    fn unknown_and_malformed_lines_are_ignored() {
        let mut p = Parser::default();
        for l in [
            "",
            "garbage",
            "2026/11/13 20:00:00 1 2 [INFO Client 1] : You have entered ",
            "2026/11/13 20:00:00 1 2 [INFO Client 1] : X (Y) is now level abc",
            "2026/11/13 20:00:00 1 2 [INFO Client 1] Generating level x area \"1_1_town\" with seed 1",
            "2026/11/13 20:00:00 1 2 [DEBUG Engine 1] : You have entered Foo.",
        ] {
            assert!(p.feed(l).is_empty(), "{l}");
        }
    }
}
