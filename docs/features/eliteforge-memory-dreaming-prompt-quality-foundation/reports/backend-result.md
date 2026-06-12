# Summary

Implemented the minimal script-contract slice for AC-MDPQ-01, AC-MDPQ-02, AC-MDPQ-03, and AC-MDPQ-07.

# Evidence Rows

- AC-MDPQ-01 | red: `make skill-memory-dreaming-test` failed after focused tests were added because prompt reference files and task-detail prompt metadata were missing | green: `make skill-memory-dreaming-test` passed with promptVersion, promptHash, taskPrompt, outputSchema, requirements, and previewPolicy for all task types | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | browser:not applicable | status: pass.

- AC-MDPQ-02 | red: `make skill-memory-dreaming-test` failed after focused tests were added because V2 provenance, evidence, noPointReason, and point-level REM were missing | green: `make skill-memory-dreaming-test` passed with V2 provenance, prompt hash/version, resolvable EvidenceRef, empty Light handling, and REM-per-point scheduling | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | browser:not applicable | status: pass.

- AC-MDPQ-03 | red: `make skill-memory-dreaming-test` failed after focused tests were added because previewPolicy, default redaction, and generated-root ignores were missing | green: `make skill-memory-dreaming-test` passed with redacted previews, `--raw-preview`, cleanup-temp, temp outputs under `/tmp`, and `.intentforge-dreaming/` ignored | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | browser:not applicable | status: pass.

- AC-MDPQ-07 | red: `make skill-memory-dreaming-test` failed after focused tests were added because REM binding and Memory MCP rejection were missing | green: `make skill-memory-dreaming-test` passed with REM bound to sourcePointId, Deep bound to REM deep-request refs, and nonzero memoryMcpCalls rejected | command_ref: CMD-SKILL-MEMORY-DREAMING-TEST | browser:not applicable | status: pass.

# Changed Paths

- `.gitignore`
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming.py`
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib/prompt_contract.py`
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib/schemas.py`
- `tron-skills/eliteforge-memory-dreaming/scripts/memory_dreaming_lib/tasks.py`
- `tron-skills/eliteforge-memory-dreaming/references/prompts/`
- `tron-skills/eliteforge-memory-dreaming/references/schemas/`
- `tron-skills/eliteforge-memory-dreaming/tests/test_memory_dreaming_cli.py`

# Commands

- RED: `make skill-memory-dreaming-test` -> failed, 17 tests run, 4 failures and 1 error from missing script-contract behavior.
- GREEN: `make skill-memory-dreaming-test` -> passed, 17 tests run.
- Supplemental: `git diff --check` -> passed.
- Supplemental: `python3 -m compileall -q tron-skills/eliteforge-memory-dreaming/scripts` -> passed; generated `__pycache__` directories were removed.

# Blockers

- None.
