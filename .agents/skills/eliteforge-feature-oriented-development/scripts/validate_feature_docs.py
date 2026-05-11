#!/usr/bin/env python3
"""Validate EliteForge multi-agent feature-oriented TDD workspaces."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Iterable


SPEC_INITIAL_FILES = {
    "spec/feature-spec.md": [
        "Initial Agent Draft",
        "Feature Identity",
        "Requirement Description",
        "Goals And Non-Goals",
        "User Roles And Permissions",
        "Page Descriptions",
        "Domain Model",
        "Module And Boundary Map",
        "Repository Findings",
        "Feature Dependencies",
        "Open Decisions",
    ],
    "spec/database-design.md": [
        "Initial Agent Draft",
        "Database Decision",
        "Existing Schema Impact",
        "Proposed Schema",
        "Relationships And Cardinality",
        "Migration Plan",
        "Query And Performance Plan",
        "Data Governance",
        "Test Data And Fixtures",
        "Database Validation Commands",
        "Open Decisions",
    ],
    "spec/api-contract.md": [
        "Initial Agent Draft",
        "API Overview",
        "Endpoints",
        "Request And Response DTOs",
        "Auth And Permissions",
        "Error Contract",
        "Frontend Backend Boundary",
        "Database And Transaction Boundary",
        "Mock Or Adapter Expectations",
        "Open Decisions",
    ],
    "spec/acceptance.md": [
        "Initial Agent Draft",
        "Acceptance Criteria Matrix",
        "Business Acceptance",
        "UI Acceptance",
        "Backend Acceptance",
        "Database Acceptance",
        "Integration Acceptance",
        "Security And Privacy Acceptance",
        "Regression Acceptance",
        "Evidence Requirements",
    ],
    "spec/test-scope.md": [
        "Initial Agent Draft",
        "Scope Decision",
        "Recommended Scope",
        "Included Test Layers",
        "Excluded Test Layers",
        "Environment Requirements",
        "Test Boundary Matrix",
        "Regression Command Candidates",
        "Evidence Required For Completion",
        "Open Decisions",
    ],
}

SPEC_COMPLETE_FILES = {
    **SPEC_INITIAL_FILES,
    "spec/test-plan.md": [
        "Test Plan Decision",
        "Test Strategy",
        "Positive Scenarios",
        "Exception Scenarios",
        "Boundary Scenarios",
        "TDD Cycle Table",
        "TDD Cycle Details",
        "Unit Test Plan",
        "Integration Test Plan",
        "Database Test Plan",
        "Frontend Test Plan",
        "Browser Acceptance Plan",
        "Security And Privacy Test Plan",
        "Test Data And Fixtures",
        "Regression Scope",
        "Command Matrix",
    ],
}

SPEC_STAGE_FILES = {
    "feature": [
        "spec/feature-spec.md",
    ],
    "database": [
        "spec/feature-spec.md",
        "spec/database-design.md",
    ],
    "api": [
        "spec/feature-spec.md",
        "spec/database-design.md",
        "spec/api-contract.md",
    ],
    "acceptance": [
        "spec/feature-spec.md",
        "spec/database-design.md",
        "spec/api-contract.md",
        "spec/acceptance.md",
        "spec/test-scope.md",
    ],
}

ORCHESTRATION_FILES = {
    "orchestration/worktree-plan.md": [
        "Main Agent",
        "Repository State",
        "Integration Branch",
        "Agent Worktrees",
        "Branch And Path Rules",
        "Merge Strategy",
        "Conflict Rules",
        "Cleanup Plan",
        "Open Decisions",
    ],
    "orchestration/integration-log.md": [
        "Summary",
        "Integration Events",
        "Conflict Log",
        "Validation After Integration",
        "Blockers",
    ],
}

TASK_FILES = {
    "tasks/database-task.md": "reports/database-result.md",
    "tasks/backend-task.md": "reports/backend-result.md",
    "tasks/frontend-task.md": "reports/frontend-result.md",
    "tasks/test-task.md": "reports/test-report.md",
    "tasks/review-task.md": "reports/review-report.md",
}

TASK_HEADINGS = ["Execution Contract", "TDD Contract", "Task Table", "Worktree Notes", "Integration Notes", "Open Decisions"]
TASK_PHASES = {"red", "green", "refactor", "review", "docs", "explore", "discovery", "support", "verify", "integrate"}
FINAL_TASK_STATUSES = {"done", "not_applicable", "skipped_approved"}
READY_TASK_STATUSES = {"ready", "done", "not_applicable", "skipped_approved"}

REPORT_FILES = {
    "reports/database-result.md": [
        "Summary",
        "Task Results",
        "TDD Evidence",
        "Worktree Evidence",
        "Changed Paths",
        "Commands",
        "Acceptance Mapping",
        "Blockers",
    ],
    "reports/backend-result.md": [
        "Summary",
        "Task Results",
        "TDD Evidence",
        "Worktree Evidence",
        "Changed Paths",
        "Commands",
        "Acceptance Mapping",
        "Blockers",
    ],
    "reports/frontend-result.md": [
        "Summary",
        "Task Results",
        "TDD Evidence",
        "Worktree Evidence",
        "Changed Paths",
        "Commands",
        "Acceptance Mapping",
        "Blockers",
    ],
    "reports/test-report.md": [
        "Summary",
        "Test Task Evidence",
        "Cycle Evidence",
        "RED Failure Evidence",
        "GREEN Pass Evidence",
        "Refactor And Regression Evidence",
        "Changed Test Assets",
        "Worktree Evidence",
        "Acceptance Mapping",
        "Blockers",
    ],
    "reports/worktree-report.md": [
        "Summary",
        "Worktree Registry",
        "Branch Registry",
        "Integration Evidence",
        "Validation Evidence",
        "Blockers",
    ],
    "reports/review-report.md": [
        "Summary",
        "Task Results",
        "Feature Docs Validation",
        "Database Design Review",
        "Test Scope Review",
        "Requirement-To-Test Traceability",
        "Worktree Integration Review",
        "Implementation Review",
        "Acceptance Evidence",
        "TDD Evidence Review",
        "Remaining Risks",
        "Final Status",
    ],
}

DOWNSTREAM_ARTIFACT_DIRS = ("tasks", "orchestration", "reports")

FINAL_MARKERS = re.compile(
    r"\b(TODO|TBD|FIXME)\b|waiting_human_decision|待确认|未定|Not run|Not reviewed",
    re.IGNORECASE,
)
AC_ID_RE = re.compile(r"\bAC-[A-Z0-9]+-\d{2,}\b")
TDD_ID_RE = re.compile(r"\bTDD-[A-Z0-9]+-\d{2,}\b")
PLACEHOLDER_VALUES = {"", "-", "none", "n/a", "tbd", "todo", "unknown", "not discovered"}


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text()


def headings(text: str) -> set[str]:
    found: set[str] = set()
    for line in text.splitlines():
        match = re.match(r"^#{1,6}\s+(.+?)\s*$", line)
        if match:
            found.add(match.group(1).strip())
    return found


def section(text: str, title: str) -> str:
    lines = text.splitlines()
    capture = False
    body: list[str] = []
    current_level = None
    for line in lines:
        match = re.match(r"^(#{1,6})\s+(.+?)\s*$", line)
        if match and match.group(2).strip() == title:
            capture = True
            current_level = len(match.group(1))
            continue
        if capture and match and len(match.group(1)) <= (current_level or 6):
            break
        if capture:
            body.append(line)
    return "\n".join(body).strip()


def key_value(section_text: str, key: str) -> str:
    pattern = re.compile(rf"^-?\s*{re.escape(key)}\s*:\s*(.+?)\s*$", re.IGNORECASE | re.MULTILINE)
    match = pattern.search(section_text)
    return match.group(1).strip() if match else ""


def normalized(value: str) -> str:
    return value.strip().strip("`").lower()


def is_placeholder(value: str) -> bool:
    return normalized(value) in PLACEHOLDER_VALUES


def is_template_placeholder(value: str) -> bool:
    return "{{" in value or "}}" in value or is_placeholder(value)


def check_file(root: Path, rel: str, required_headings: Iterable[str], errors: list[str]) -> str:
    path = root / rel
    if not path.is_file():
        errors.append(f"missing file: {rel}")
        return ""
    text = read_text(path)
    found = headings(text)
    for heading in required_headings:
        if heading not in found:
            errors.append(f"{rel}: missing heading '{heading}'")
    return text


def table_body_rows(text: str, title: str) -> list[list[str]]:
    rows: list[list[str]] = []
    in_table = False
    for line in section(text, title).splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            if in_table:
                break
            continue
        in_table = True
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if not cells or set(cells[0]) <= {"-"}:
            continue
        header_tokens = {
            "acceptance id",
            "cycle id",
            "command",
            "task id",
            "agent id",
            "event id",
            "conflict id",
            "object",
            "id",
            "layer",
            "scope",
            "dependency feature name",
        }
        if cells[0].lower() in header_tokens:
            continue
        rows.append(cells)
    return rows


def acceptance_ids_from_acceptance(text: str) -> set[str]:
    return set(AC_ID_RE.findall(section(text, "Acceptance Criteria Matrix") or text))


def acceptance_ids_from_cell(cell: str) -> set[str]:
    return set(AC_ID_RE.findall(cell))


def tdd_ids_from_text(text: str) -> set[str]:
    return set(TDD_ID_RE.findall(text))


def id_module(identifier: str) -> str:
    parts = identifier.split("-")
    return parts[1] if len(parts) >= 3 else ""


def tdd_cycle_rows(text: str) -> list[list[str]]:
    rows: list[list[str]] = []
    for section_title in ("Positive Scenarios", "Exception Scenarios", "Boundary Scenarios"):
        rows.extend(table_body_rows(text, section_title))
    return rows


def check_tdd_cycle_module_alignment(text: str, errors: list[str]) -> None:
    for cells in tdd_cycle_rows(text):
        if len(cells) < 2:
            continue
        cycle_id = cells[0]
        primary_ac = cells[1]
        if cycle_id and not TDD_ID_RE.fullmatch(cycle_id):
            errors.append(f"spec/test-plan.md: cycle id '{cycle_id}' must use TDD-<MODULE>-NN format")
            continue
        if primary_ac and not AC_ID_RE.fullmatch(primary_ac):
            errors.append(f"spec/test-plan.md: primary acceptance id '{primary_ac}' must use AC-<MODULE>-NN format")
            continue
        if cycle_id and primary_ac and id_module(cycle_id) != id_module(primary_ac):
            errors.append(
                f"spec/test-plan.md: cycle id '{cycle_id}' module must match primary acceptance id '{primary_ac}'"
            )


def check_feature_dependencies(text: str, errors: list[str]) -> None:
    dependency_section = section(text, "Feature Dependencies")
    if not dependency_section.strip():
        errors.append("spec/feature-spec.md: Feature Dependencies must record strong feature dependencies or an explicit none-found scan result")
        return

    if re.search(r"Strong feature dependencies:\s*none found", dependency_section, re.IGNORECASE):
        return

    for cells in table_body_rows(text, "Feature Dependencies"):
        if len(cells) < 2:
            continue
        dependency_name = cells[0]
        feature_slug = cells[1]
        if not is_template_placeholder(dependency_name) and not is_template_placeholder(feature_slug):
            return

    errors.append(
        "spec/feature-spec.md: Feature Dependencies must list strong dependency feature names or state 'Strong feature dependencies: none found...'"
    )


def parse_task_rows(text: str, rel: str) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for cells in table_body_rows(text, "Task Table"):
        if not cells:
            continue
        if len(cells) < 11:
            rows.append({"id": cells[0] if cells else "", "_invalid": f"expected 11 columns, found {len(cells)}", "_file": rel})
            continue
        rows.append(
            {
                "id": cells[0],
                "tdd_phase": cells[1].lower(),
                "owner": cells[2],
                "agent_role": cells[3],
                "dependencies": cells[4],
                "worktree": cells[5],
                "write_scope": cells[6],
                "validation": cells[7],
                "acceptance_ids": cells[8],
                "report_target": cells[9],
                "status": cells[10].lower(),
                "_file": rel,
            }
        )
    return rows


def dependency_ids(cell: str) -> set[str]:
    if normalized(cell) in {"none", "n/a", "-"}:
        return set()
    return {dep.strip() for dep in re.split(r"[,;\s]+", cell) if dep.strip()}


def check_decision_statuses(root: Path, errors: list[str], require_approved: bool) -> None:
    db_text = read_text(root / "spec/database-design.md") if (root / "spec/database-design.md").exists() else ""
    test_scope_text = read_text(root / "spec/test-scope.md") if (root / "spec/test-scope.md").exists() else ""
    test_plan_text = read_text(root / "spec/test-plan.md") if (root / "spec/test-plan.md").exists() else ""

    if db_text:
        db_status = normalized(key_value(section(db_text, "Database Decision"), "status"))
        if require_approved and db_status not in {"approved", "not_applicable"}:
            errors.append("spec/database-design.md: Database Decision status must be approved or not_applicable before task execution")
    if test_scope_text:
        test_scope_status = normalized(key_value(section(test_scope_text, "Scope Decision"), "status"))
        if require_approved and test_scope_status not in {"approved", "approved_with_defaults"}:
            errors.append("spec/test-scope.md: Scope Decision status must be approved or approved_with_defaults before task execution")
    if test_plan_text:
        test_plan_status = normalized(key_value(section(test_plan_text, "Test Plan Decision"), "status"))
        if require_approved and test_plan_status not in {"approved", "approved_with_defaults"}:
            errors.append("spec/test-plan.md: Test Plan Decision status must be approved or approved_with_defaults before task breakdown")


def check_specs(root: Path, errors: list[str], complete: bool, require_approved: bool, spec_stage: str) -> tuple[set[str], set[str]]:
    if complete:
        selected = SPEC_COMPLETE_FILES
    else:
        selected = {rel: SPEC_INITIAL_FILES[rel] for rel in SPEC_STAGE_FILES[spec_stage]}

    texts: dict[str, str] = {}
    for rel, required in selected.items():
        texts[rel] = check_file(root, rel, required, errors)

    feature_spec = texts.get("spec/feature-spec.md", "")
    if feature_spec:
        check_feature_dependencies(feature_spec, errors)

    ac_ids: set[str] = set()
    tdd_ids: set[str] = set()
    acceptance = texts.get("spec/acceptance.md", "")
    if acceptance:
        ac_ids = acceptance_ids_from_acceptance(acceptance)
        if not ac_ids:
            errors.append("spec/acceptance.md: must contain at least one stable acceptance id like AC-MODULE-01")

    if complete:
        test_plan = texts.get("spec/test-plan.md", "")
        if test_plan:
            tdd_ids = tdd_ids_from_text(test_plan)
            if not tdd_ids:
                errors.append("spec/test-plan.md: must contain at least one TDD cycle id like TDD-MODULE-01")
            if not tdd_cycle_rows(test_plan):
                errors.append("spec/test-plan.md: TDD Cycle Table must contain at least one cycle row")
            check_tdd_cycle_module_alignment(test_plan, errors)
            unknown_acs = set(AC_ID_RE.findall(test_plan)) - ac_ids
            if ac_ids and unknown_acs:
                errors.append(f"spec/test-plan.md: references unknown acceptance ids: {', '.join(sorted(unknown_acs))}")
        check_decision_statuses(root, errors, require_approved=require_approved)

    return ac_ids, tdd_ids


def check_orchestration(root: Path, errors: list[str], final: bool) -> None:
    texts: dict[str, str] = {}
    for rel, required in ORCHESTRATION_FILES.items():
        texts[rel] = check_file(root, rel, required, errors)

    worktree_plan = texts.get("orchestration/worktree-plan.md", "")
    if worktree_plan:
        agent_rows = table_body_rows(worktree_plan, "Agent Worktrees")
        if not agent_rows:
            errors.append("orchestration/worktree-plan.md: Agent Worktrees must contain at least one worker worktree row")
        integration = section(worktree_plan, "Integration Branch")
        branch = key_value(integration, "branch")
        if not branch or "eliteforge/integration/" not in branch:
            errors.append("orchestration/worktree-plan.md: Integration Branch must use eliteforge/integration/<feature-slug>")
        main = section(worktree_plan, "Main Agent")
        status = normalized(key_value(main, "status"))
        if final and status not in {"done", "approved", "complete"}:
            errors.append("orchestration/worktree-plan.md: Main Agent status must be done/approved/complete for final validation")

    integration_log = texts.get("orchestration/integration-log.md", "")
    if final and integration_log:
        if not table_body_rows(integration_log, "Integration Events"):
            errors.append("orchestration/integration-log.md: Integration Events must contain at least one integration row for final validation")
        if section_has_unresolved_items(integration_log, "Blockers"):
            errors.append("orchestration/integration-log.md: Blockers must be empty or explicitly 'None' for final validation")
        if FINAL_MARKERS.search(integration_log):
            errors.append("orchestration/integration-log.md: contains unresolved placeholder or waiting marker")


def check_tasks(root: Path, errors: list[str], ac_ids: set[str], final: bool) -> dict[str, list[dict[str, str]]]:
    all_rows: dict[str, list[dict[str, str]]] = {}
    flat_rows: list[dict[str, str]] = []
    for rel, report_target in TASK_FILES.items():
        text = check_file(root, rel, TASK_HEADINGS, errors)
        if not text:
            continue
        rows = parse_task_rows(text, rel)
        if not rows:
            errors.append(f"{rel}: Task Table must contain at least one task row")
        all_rows[rel] = rows
        flat_rows.extend(rows)

    red_ids = {row.get("id", "") for row in flat_rows if row.get("tdd_phase") == "red" and not row.get("_invalid")}
    all_ids = {row.get("id", "") for row in flat_rows if row.get("id") and not row.get("_invalid")}
    ac_to_red: dict[str, set[str]] = {ac: set() for ac in ac_ids}
    ac_to_green_or_verify: dict[str, set[str]] = {ac: set() for ac in ac_ids}
    seen_ids: set[str] = set()

    for row in flat_rows:
        rel = row.get("_file", "")
        task_id = row.get("id", "")
        if row.get("_invalid"):
            errors.append(f"{rel}: task row '{task_id}' {row['_invalid']}")
            continue
        if task_id in seen_ids:
            errors.append(f"{rel}: duplicate task id '{task_id}'")
        seen_ids.add(task_id)

        report_target = TASK_FILES.get(rel)
        for field in ("id", "tdd_phase", "owner", "agent_role", "worktree", "write_scope", "validation", "report_target", "status"):
            if is_placeholder(row.get(field, "")):
                errors.append(f"{rel}: task row '{task_id}' missing {field.replace('_', ' ')}")
        worktree_value = row.get("worktree", "")
        agent_role_value = normalized(row.get("agent_role", ""))
        if agent_role_value != "lead" and worktree_value and "eliteforge/" not in worktree_value:
            errors.append(f"{rel}: task row '{task_id}' worktree must use eliteforge/<agent-id>/<feature-slug> order")
        if "eliteforge/" in worktree_value:
            after = worktree_value.split("eliteforge/", 1)[1].strip("/")
            parts = [part for part in after.split("/") if part]
            if len(parts) < 2:
                errors.append(f"{rel}: task row '{task_id}' worktree must include both <agent-id> and <feature-slug> after eliteforge/")
            elif parts[0] in {"integration", "<feature-slug>", row.get("acceptance_ids", "")}:
                errors.append(f"{rel}: task row '{task_id}' worktree must put <agent-id> before <feature-slug>")

        if row.get("tdd_phase") and row["tdd_phase"] not in TASK_PHASES:
            errors.append(f"{rel}: task row '{task_id}' tdd phase must be one of {', '.join(sorted(TASK_PHASES))}")
        if row.get("report_target") and row["report_target"] != report_target:
            errors.append(f"{rel}: task row '{task_id}' report target must be {report_target}")
        if row.get("tdd_phase") == "red" and rel != "tasks/test-task.md":
            errors.append(f"{rel}: RED task row '{task_id}' must be placed in tasks/test-task.md")
        if row.get("tdd_phase") == "green":
            deps = dependency_ids(row.get("dependencies", ""))
            if not (deps & red_ids):
                errors.append(f"{rel}: GREEN task row '{task_id}' must depend on at least one RED task row")
        deps = dependency_ids(row.get("dependencies", ""))
        unknown_deps = deps - all_ids
        if unknown_deps:
            errors.append(f"{rel}: task row '{task_id}' references unknown dependencies: {', '.join(sorted(unknown_deps))}")

        row_acs = acceptance_ids_from_cell(row.get("acceptance_ids", ""))
        if row.get("tdd_phase") not in {"docs", "explore", "integrate"} and not row_acs:
            errors.append(f"{rel}: task row '{task_id}' must reference acceptance ids")
        unknown_acs = row_acs - ac_ids
        if ac_ids and unknown_acs:
            errors.append(f"{rel}: task row '{task_id}' references unknown acceptance ids: {', '.join(sorted(unknown_acs))}")
        for ac in row_acs:
            if row.get("tdd_phase") == "red":
                ac_to_red.setdefault(ac, set()).add(task_id)
            if row.get("tdd_phase") in {"green", "verify", "review"}:
                ac_to_green_or_verify.setdefault(ac, set()).add(task_id)

        status = normalized(row.get("status", ""))
        if final and status not in FINAL_TASK_STATUSES:
            errors.append(f"{rel}: task row '{task_id}' status must be done/not_applicable/skipped_approved for final validation")
        if not final and status and status not in READY_TASK_STATUSES and status not in {"draft", "blocked", "waiting_human_decision", "in_progress"}:
            errors.append(f"{rel}: task row '{task_id}' has non-standard status '{row.get('status')}'")

    for ac in sorted(ac_ids):
        if not ac_to_red.get(ac):
            errors.append(f"tasks: acceptance id {ac} has no RED test task")
        if not ac_to_green_or_verify.get(ac):
            errors.append(f"tasks: acceptance id {ac} has no GREEN/VERIFY/REVIEW task")

    return all_rows


def report_task_ids(text: str, section_title: str) -> set[str]:
    ids: set[str] = set()
    for cells in table_body_rows(text, section_title):
        if cells:
            ids.add(cells[0])
    return ids


def section_has_unresolved_items(text: str, title: str) -> bool:
    body = section(text, title)
    meaningful = [
        line.strip()
        for line in body.splitlines()
        if line.strip()
        and not re.match(r"^[-*]?\s*(None|No unresolved .+|No blockers|No remaining risks)\.?$", line.strip(), re.IGNORECASE)
        and not line.strip().startswith("|")
    ]
    return bool(meaningful)


def check_agent_reports(root: Path, errors: list[str], final: bool) -> None:
    agents_dir = root / "reports" / "agents"
    if not agents_dir.exists():
        errors.append("reports/agents: missing directory for subagent report fragments")
        return
    fragments = list(agents_dir.glob("*.md"))
    if final and not fragments:
        errors.append("reports/agents: must contain at least one subagent report fragment for final validation")
    for fragment in fragments:
        text = read_text(fragment)
        for heading in ["Agent Identity", "Scope", "Task Results", "TDD Evidence", "Changed Paths", "Commands", "Blockers"]:
            if heading not in headings(text):
                errors.append(f"reports/agents/{fragment.name}: missing heading '{heading}'")
        if final and FINAL_MARKERS.search(text):
            errors.append(f"reports/agents/{fragment.name}: contains unresolved placeholder or waiting marker")


def check_reports(root: Path, task_rows: dict[str, list[dict[str, str]]], errors: list[str], ac_ids: set[str], tdd_ids: set[str], final: bool) -> None:
    report_texts: dict[str, str] = {}
    for rel, required in REPORT_FILES.items():
        report_texts[rel] = check_file(root, rel, required, errors)

    for task_file, rows in task_rows.items():
        report_rel = TASK_FILES[task_file]
        text = report_texts.get(report_rel, "")
        if not text:
            continue
        task_section = "Test Task Evidence" if report_rel == "reports/test-report.md" else "Task Results"
        reported = report_task_ids(text, task_section)
        for row in rows:
            task_id = row.get("id", "")
            if task_id and not row.get("_invalid") and task_id not in reported:
                errors.append(f"{report_rel}: missing {task_section} row for task '{task_id}' from {task_file}")

    for rel, text in report_texts.items():
        if not text:
            continue
        if final:
            if FINAL_MARKERS.search(text):
                errors.append(f"{rel}: contains unresolved placeholder or waiting marker")
            if rel.endswith("review-report.md"):
                if section_has_unresolved_items(text, "Remaining Risks"):
                    errors.append(f"{rel}: Remaining Risks must be empty or explicitly 'None' for final validation")
            elif section_has_unresolved_items(text, "Blockers"):
                errors.append(f"{rel}: Blockers must be empty or explicitly 'None' for final validation")
        if rel not in {"reports/review-report.md", "reports/worktree-report.md"}:
            if rel == "reports/test-report.md":
                tdd_source = "\n".join(
                    section(text, title)
                    for title in (
                        "Cycle Evidence",
                        "RED Failure Evidence",
                        "GREEN Pass Evidence",
                        "Refactor And Regression Evidence",
                    )
                )
            else:
                tdd_source = section(text, "TDD Evidence")
            tdd_in_report = tdd_ids_from_text(tdd_source)
            if final and not tdd_in_report:
                errors.append(f"{rel}: must reference at least one TDD cycle id")
            if tdd_ids and tdd_in_report - tdd_ids:
                errors.append(f"{rel}: TDD Evidence references unknown TDD ids: {', '.join(sorted(tdd_in_report - tdd_ids))}")
            if final and not table_body_rows(text, "Worktree Evidence"):
                errors.append(f"{rel}: Worktree Evidence must contain at least one row for final validation")
        if rel != "reports/worktree-report.md" and "Acceptance Mapping" in headings(text):
            report_acs = set(AC_ID_RE.findall(section(text, "Acceptance Mapping")))
            if final and ac_ids and not report_acs:
                errors.append(f"{rel}: Acceptance Mapping must reference acceptance ids like AC-MODULE-01")
            if ac_ids and report_acs - ac_ids:
                errors.append(f"{rel}: Acceptance Mapping references unknown acceptance ids: {', '.join(sorted(report_acs - ac_ids))}")

    check_agent_reports(root, errors, final=final)


def check_no_downstream_artifacts(root: Path, errors: list[str]) -> None:
    for rel in DOWNSTREAM_ARTIFACT_DIRS:
        path = root / rel
        if path.exists() and any(path.iterdir()):
            errors.append(f"{rel}: downstream artifacts must not be created during the spec/test-plan gate")


def validate(root: Path, phase: str, complete_spec: bool, spec_stage: str, no_downstream_artifacts: bool) -> list[str]:
    errors: list[str] = []
    require_complete = complete_spec or phase in {"tasks", "worktrees", "reports", "final"}
    require_approved = phase in {"tasks", "worktrees", "reports", "final"}
    ac_ids, tdd_ids = check_specs(
        root,
        errors,
        complete=require_complete,
        require_approved=require_approved,
        spec_stage=spec_stage,
    )

    if no_downstream_artifacts:
        if phase != "spec":
            errors.append("--no-downstream-artifacts is only valid with --phase spec")
        check_no_downstream_artifacts(root, errors)

    if phase in {"tasks", "worktrees", "reports", "final"}:
        check_orchestration(root, errors, final=phase == "final")

    task_rows: dict[str, list[dict[str, str]]] = {}
    if phase in {"tasks", "worktrees", "reports", "final"}:
        task_rows = check_tasks(root, errors, ac_ids=ac_ids, final=phase == "final")

    if phase in {"reports", "final"}:
        check_reports(root, task_rows, errors, ac_ids=ac_ids, tdd_ids=tdd_ids, final=phase == "final")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate an EliteForge multi-agent feature-oriented TDD workspace.")
    parser.add_argument("feature_dir", type=Path, help="Path to docs/features/<feature-slug>")
    parser.add_argument("--phase", choices=["spec", "tasks", "worktrees", "reports", "final"], default="final")
    parser.add_argument(
        "--spec-stage",
        choices=["feature", "database", "api", "acceptance"],
        default="feature",
        help="When --phase spec is used, validate only the current progressive design gate.",
    )
    parser.add_argument(
        "--complete-spec",
        action="store_true",
        help="When --phase spec is used, require all progressive spec files plus test-plan.",
    )
    parser.add_argument(
        "--no-downstream-artifacts",
        action="store_true",
        help="When validating a 0-1 spec/test-plan gate, fail if tasks, orchestration, or reports were created early.",
    )
    args = parser.parse_args()

    root = args.feature_dir
    if not root.is_dir():
        print(f"ERROR: feature directory not found: {root}", file=sys.stderr)
        return 2

    errors = validate(root, args.phase, args.complete_spec, args.spec_stage, args.no_downstream_artifacts)
    if errors:
        print("Feature docs validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1

    print(f"Feature docs validation OK: {root} ({args.phase})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
