use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Wildcard,
    Regex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutingRule {
    pub kind: PatternKind,
    pub pattern: String,
    pub browser_path: String,
    pub browser_arguments: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserTarget<'a> {
    pub path: &'a str,
    pub arguments: &'a str,
}

impl RoutingRule {
    pub fn validate(&self) -> Result<(), String> {
        if self.pattern.is_empty() {
            return Err("Vzor pravidla nesmí být prázdný.".to_owned());
        }
        if self.browser_path.is_empty() {
            return Err("Pravidlo musí mít vybraný prohlížeč.".to_owned());
        }
        self.compiled_pattern()
            .map(|_| ())
            .map_err(|error| format!("Neplatný regulární výraz: {error}"))
    }

    pub fn matches(&self, url: &str) -> bool {
        self.compiled_pattern()
            .is_ok_and(|pattern| pattern.is_match(url))
    }

    fn compiled_pattern(&self) -> Result<Regex, regex::Error> {
        match self.kind {
            PatternKind::Wildcard => Regex::new(&wildcard_to_regex(&self.pattern)),
            PatternKind::Regex => Regex::new(&self.pattern),
        }
    }
}

pub fn resolve<'a>(
    rules: &'a [RoutingRule],
    default_path: &'a str,
    default_arguments: &'a str,
    url: &str,
) -> BrowserTarget<'a> {
    rules.iter().find(|rule| rule.matches(url)).map_or(
        BrowserTarget {
            path: default_path,
            arguments: default_arguments,
        },
        |rule| BrowserTarget {
            path: &rule.browser_path,
            arguments: &rule.browser_arguments,
        },
    )
}

pub fn encode_rules(rules: &[RoutingRule]) -> String {
    let mut encoded = String::new();
    for rule in rules {
        encoded.push(match rule.kind {
            PatternKind::Wildcard => 'W',
            PatternKind::Regex => 'R',
        });
        push_field(&mut encoded, &rule.pattern);
        push_field(&mut encoded, &rule.browser_path);
        push_field(&mut encoded, &rule.browser_arguments);
    }
    encoded
}

pub fn decode_rules(encoded: &str) -> Result<Vec<RoutingRule>, String> {
    let mut cursor = 0;
    let mut rules = Vec::new();
    while cursor < encoded.len() {
        let kind = match encoded.as_bytes()[cursor] {
            b'W' => PatternKind::Wildcard,
            b'R' => PatternKind::Regex,
            _ => return Err("Neznámý typ uloženého pravidla.".to_owned()),
        };
        cursor += 1;
        let pattern = read_field(encoded, &mut cursor)?;
        let browser_path = read_field(encoded, &mut cursor)?;
        let browser_arguments = read_field(encoded, &mut cursor)?;
        let rule = RoutingRule {
            kind,
            pattern,
            browser_path,
            browser_arguments,
        };
        rule.validate()?;
        rules.push(rule);
    }
    Ok(rules)
}

fn wildcard_to_regex(pattern: &str) -> String {
    let mut regex = String::from("^");
    for (index, part) in pattern.split('*').enumerate() {
        if index > 0 {
            regex.push_str(".*");
        }
        regex.push_str(&regex::escape(part));
    }
    regex.push('$');
    regex
}

fn push_field(target: &mut String, value: &str) {
    target.push_str(&value.len().to_string());
    target.push(':');
    target.push_str(value);
}

fn read_field(source: &str, cursor: &mut usize) -> Result<String, String> {
    let length_end = source[*cursor..]
        .find(':')
        .map(|offset| *cursor + offset)
        .ok_or_else(|| "Poškozená délka uloženého pravidla.".to_owned())?;
    let length = source[*cursor..length_end]
        .parse::<usize>()
        .map_err(|_| "Poškozená délka uloženého pravidla.".to_owned())?;
    let start = length_end + 1;
    let end = start
        .checked_add(length)
        .filter(|end| *end <= source.len())
        .ok_or_else(|| "Neúplné uložené pravidlo.".to_owned())?;
    let value = source
        .get(start..end)
        .ok_or_else(|| "Poškozené UTF-8 v uloženém pravidle.".to_owned())?;
    *cursor = end;
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{PatternKind, RoutingRule, decode_rules, encode_rules, resolve};

    fn rule(kind: PatternKind, pattern: &str, browser: &str) -> RoutingRule {
        RoutingRule {
            kind,
            pattern: pattern.to_owned(),
            browser_path: browser.to_owned(),
            browser_arguments: String::new(),
        }
    }

    #[test]
    fn wildcard_matches_the_whole_url_and_treats_other_characters_literally() {
        let rule = rule(PatternKind::Wildcard, "https://*.company.cz/*", "work.exe");
        assert!(rule.matches("https://portal.company.cz/home"));
        assert!(!rule.matches("https://portalXcompany.cz/home"));
    }

    #[test]
    fn regex_is_validated() {
        let rule = rule(PatternKind::Regex, "[", "browser.exe");
        assert!(rule.validate().is_err());
    }

    #[test]
    fn first_matching_rule_wins_and_default_is_the_fallback() {
        let rules = vec![
            rule(PatternKind::Wildcard, "*company.cz*", "first.exe"),
            rule(PatternKind::Regex, "company\\.cz", "second.exe"),
        ];
        assert_eq!(
            resolve(&rules, "default.exe", "", "https://company.cz").path,
            "first.exe"
        );
        assert_eq!(
            resolve(&rules, "default.exe", "", "https://private.cz").path,
            "default.exe"
        );
    }

    #[test]
    fn persistence_round_trip_preserves_unicode_and_separators() {
        let rules = vec![RoutingRule {
            kind: PatternKind::Regex,
            pattern: "https://firma.cz/(číslo|:)*".to_owned(),
            browser_path: r"C:\Prohlížeče\Fire|fox.exe".to_owned(),
            browser_arguments: "--new-window {url}".to_owned(),
        }];
        assert_eq!(decode_rules(&encode_rules(&rules)).unwrap(), rules);
    }
}
