use crate::{ExecutionMode, Skill, SkillSource};
use std::path::{Path, PathBuf};
use tracing::warn;

pub struct SkillLoader {
    search_paths: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    pub fn load_all(&self) -> Vec<Skill> {
        let mut skills = Vec::new();
        skills.extend(self.load_bundled());
        for path in &self.search_paths {
            skills.extend(self.load_from_directory(path));
        }
        skills
    }

    fn load_bundled(&self) -> Vec<Skill> {
        vec![
            Skill {
                name: "plan".into(),
                description: "Create a step-by-step plan before implementing".into(),
                content: "Before implementing, create a numbered plan with clear milestones and acceptance criteria for each step.".into(),
                execution_mode: ExecutionMode::Inline,
                source: SkillSource::Bundled,
                ..Default::default()
            },
            Skill {
                name: "code-review".into(),
                description: "Review code for bugs, security issues, and style".into(),
                content: "Review the code systematically: check for correctness, security vulnerabilities, performance issues, and adherence to project style.".into(),
                execution_mode: ExecutionMode::Fork,
                source: SkillSource::Bundled,
                ..Default::default()
            },
        ]
    }

    fn load_from_directory(&self, dir: &Path) -> Vec<Skill> {
        let mut skills = Vec::new();

        let walker = match walkdir::WalkDir::new(dir).into_iter().next() {
            Some(_) => walkdir::WalkDir::new(dir),
            None => return skills,
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if let Some(skill) = Self::parse_skill(&content, SkillSource::Filesystem) {
                        skills.push(skill);
                    }
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "failed to read skill file");
                }
            }
        }

        skills
    }

    /// Parse a skill from markdown content with YAML frontmatter.
    ///
    /// Expects content in the form:
    /// ```text
    /// ---
    /// name: my-skill
    /// description: Does a thing
    /// ---
    /// Markdown body here...
    /// ```
    pub fn parse_skill(content: &str, source: SkillSource) -> Option<Skill> {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return None;
        }

        let after_first = &trimmed[3..];
        let end_idx = after_first.find("---")?;
        let frontmatter = &after_first[..end_idx];
        let body = after_first[end_idx + 3..].trim().to_string();

        let mut skill = Skill {
            content: body,
            source,
            ..Default::default()
        };

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "name" => skill.name = value.to_string(),
                    "description" => skill.description = value.to_string(),
                    "when_to_use" => skill.when_to_use = Some(value.to_string()),
                    "model" => skill.model = Some(value.to_string()),
                    "context" => skill.context = Some(value.to_string()),
                    "execution_mode" => {
                        skill.execution_mode = match value.to_lowercase().as_str() {
                            "fork" => ExecutionMode::Fork,
                            _ => ExecutionMode::Inline,
                        };
                    }
                    "file_patterns" => {
                        skill.file_patterns = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "allowed_tools" => {
                        skill.allowed_tools = Some(
                            value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        );
                    }
                    _ => {}
                }
            }
        }

        if skill.name.is_empty() {
            return None;
        }

        Some(skill)
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_basic() {
        let md = r#"---
name: test-skill
description: A test skill
execution_mode: inline
---
This is the body."#;

        let skill = SkillLoader::parse_skill(md, SkillSource::Filesystem).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill");
        assert_eq!(skill.execution_mode, ExecutionMode::Inline);
        assert_eq!(skill.content, "This is the body.");
        assert_eq!(skill.source, SkillSource::Filesystem);
    }

    #[test]
    fn test_parse_skill_fork_mode() {
        let md = r#"---
name: review
description: Code review
execution_mode: fork
file_patterns: *.rs, *.toml
allowed_tools: read, grep
---
Review instructions."#;

        let skill = SkillLoader::parse_skill(md, SkillSource::Mcp).unwrap();
        assert_eq!(skill.name, "review");
        assert_eq!(skill.execution_mode, ExecutionMode::Fork);
        assert_eq!(skill.file_patterns, vec!["*.rs", "*.toml"]);
        assert_eq!(
            skill.allowed_tools,
            Some(vec!["read".to_string(), "grep".to_string()])
        );
        assert_eq!(skill.source, SkillSource::Mcp);
    }

    #[test]
    fn test_parse_skill_missing_name() {
        let md = r#"---
description: No name here
---
Body."#;

        assert!(SkillLoader::parse_skill(md, SkillSource::Filesystem).is_none());
    }

    #[test]
    fn test_parse_skill_no_frontmatter() {
        let md = "Just regular markdown.";
        assert!(SkillLoader::parse_skill(md, SkillSource::Filesystem).is_none());
    }

    #[test]
    fn test_parse_skill_optional_fields() {
        let md = r#"---
name: minimal
description: Bare minimum
when_to_use: When you need it
model: fast
context: project context
---
Content here."#;

        let skill = SkillLoader::parse_skill(md, SkillSource::Bundled).unwrap();
        assert_eq!(skill.when_to_use, Some("When you need it".to_string()));
        assert_eq!(skill.model, Some("fast".to_string()));
        assert_eq!(skill.context, Some("project context".to_string()));
    }

    #[test]
    fn test_load_bundled() {
        let loader = SkillLoader::new();
        let skills = loader.load_all();
        assert!(skills.len() >= 2);

        let plan = skills.iter().find(|s| s.name == "plan").unwrap();
        assert_eq!(plan.source, SkillSource::Bundled);
        assert_eq!(plan.execution_mode, ExecutionMode::Inline);

        let review = skills.iter().find(|s| s.name == "code-review").unwrap();
        assert_eq!(review.source, SkillSource::Bundled);
        assert_eq!(review.execution_mode, ExecutionMode::Fork);
    }

    #[test]
    fn test_load_from_temp_directory() {
        let dir = std::env::temp_dir().join("zero-skills-test");
        let _ = std::fs::create_dir_all(&dir);

        let skill_content = r#"---
name: temp-skill
description: From temp dir
---
Temp body."#;
        std::fs::write(dir.join("temp.md"), skill_content).unwrap();

        let mut loader = SkillLoader::new();
        loader.add_search_path(dir.clone());

        let skills = loader.load_all();
        assert!(skills.iter().any(|s| s.name == "temp-skill"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
