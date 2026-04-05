use crate::Skill;
use globset::{Glob, GlobSetBuilder};

pub struct SkillMatcher;

impl SkillMatcher {
    /// Find skills whose `file_patterns` match any of the given file paths.
    pub fn match_for_files<'a>(skills: &'a [Skill], file_paths: &[String]) -> Vec<&'a Skill> {
        skills
            .iter()
            .filter(|skill| {
                if skill.file_patterns.is_empty() {
                    return false;
                }

                let mut builder = GlobSetBuilder::new();
                for pattern in &skill.file_patterns {
                    if let Ok(glob) = Glob::new(pattern) {
                        builder.add(glob);
                    }
                }

                let set = match builder.build() {
                    Ok(s) => s,
                    Err(_) => return false,
                };

                file_paths.iter().any(|fp| set.is_match(fp))
            })
            .collect()
    }

    /// Find skills whose name or description contains the query (case-insensitive).
    pub fn match_by_query<'a>(skills: &'a [Skill], query: &str) -> Vec<&'a Skill> {
        let query_lower = query.to_lowercase();
        skills
            .iter()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query_lower)
                    || skill.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionMode, SkillSource};

    fn test_skill(name: &str, patterns: Vec<&str>, description: &str) -> Skill {
        Skill {
            name: name.into(),
            description: description.into(),
            file_patterns: patterns.into_iter().map(String::from).collect(),
            execution_mode: ExecutionMode::Inline,
            source: SkillSource::Bundled,
            ..Default::default()
        }
    }

    #[test]
    fn test_match_for_files_glob() {
        let skills = vec![
            test_skill("rust-skill", vec!["*.rs"], "Rust helper"),
            test_skill("toml-skill", vec!["*.toml"], "TOML helper"),
            test_skill("no-pattern", vec![], "No patterns"),
        ];

        let files = vec!["src/main.rs".to_string()];
        let matched = SkillMatcher::match_for_files(&skills, &files);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "rust-skill");
    }

    #[test]
    fn test_match_for_files_multiple() {
        let skills = vec![
            test_skill("web", vec!["*.ts", "*.tsx"], "Web"),
            test_skill("rust", vec!["*.rs"], "Rust"),
        ];

        let files = vec!["app.tsx".to_string(), "lib.rs".to_string()];
        let matched = SkillMatcher::match_for_files(&skills, &files);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn test_match_for_files_no_match() {
        let skills = vec![test_skill("python", vec!["*.py"], "Python")];
        let files = vec!["main.rs".to_string()];
        let matched = SkillMatcher::match_for_files(&skills, &files);
        assert!(matched.is_empty());
    }

    #[test]
    fn test_match_by_query_name() {
        let skills = vec![
            test_skill("code-review", vec![], "Review code quality"),
            test_skill("deploy", vec![], "Deploy to production"),
        ];

        let matched = SkillMatcher::match_by_query(&skills, "review");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "code-review");
    }

    #[test]
    fn test_match_by_query_description() {
        let skills = vec![
            test_skill("linter", vec![], "Check code for lint errors"),
            test_skill("deploy", vec![], "Ship to production"),
        ];

        let matched = SkillMatcher::match_by_query(&skills, "production");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "deploy");
    }

    #[test]
    fn test_match_by_query_case_insensitive() {
        let skills = vec![test_skill("Test-Skill", vec![], "Runs Tests")];

        let matched = SkillMatcher::match_by_query(&skills, "test");
        assert_eq!(matched.len(), 1);
    }
}
