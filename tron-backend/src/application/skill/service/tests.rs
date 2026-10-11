use super::*;

#[test]
fn parses_search_output_candidates() {
    let output = "\u{1b}[38;5;102mInstall with\u{1b}[0m npx skills add <owner/repo@skill>\n\
wshobson/agents@typescript-advanced-types 42K installs\n\
└ https://skills.sh/wshobson/agents/typescript-advanced-types\n\
github/awesome-copilot@javascript-typescript-jest 10.5K installs\n\
└ https://skills.sh/github/awesome-copilot/javascript-typescript-jest\n";
    let candidates = parse_search_output(output.as_bytes());
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].source_ref, "wshobson/agents");
    assert_eq!(candidates[0].skill_name, "typescript-advanced-types");
    assert_eq!(
        candidates[1].detail_url.as_deref(),
        Some("https://skills.sh/github/awesome-copilot/javascript-typescript-jest")
    );
}

#[test]
fn parses_repo_listing_output_candidates() {
    let output = "◇  Available Skills\n│\n│    eliteforge-agent-doctor\n│      检查开发环境配置\n│    eliteforge-be-fe-consensus-spec\n│      约束前后端接口共识\n└  Use --skill <name> to install specific skills\n";
    let candidates = parse_repo_listing_output(output.as_bytes(), "cloudsen/eliteforge-skills");
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].package_ref,
        "cloudsen/eliteforge-skills@eliteforge-agent-doctor"
    );
    assert_eq!(candidates[1].skill_name, "eliteforge-be-fe-consensus-spec");
}

#[test]
fn strips_ansi_sequences() {
    assert_eq!(strip_ansi("\u{1b}[38;5;102mhello\u{1b}[0m"), "hello");
}

#[test]
fn parses_skill_frontmatter_fields() {
    let frontmatter =
        parse_skill_frontmatter("---\nversion: 1.2.3\ndescription: Example skill\n---\n# Skill");
    assert_eq!(frontmatter.declared_version.as_deref(), Some("1.2.3"));
    assert_eq!(frontmatter.description.as_deref(), Some("Example skill"));
}

#[test]
fn builds_package_ref_from_source_and_skill_name() {
    assert_eq!(
        build_package_ref(
            Some("wshobson/agents".into()),
            Some("typescript-advanced-types".into())
        )
        .as_deref(),
        Some("wshobson/agents@typescript-advanced-types")
    );
}
