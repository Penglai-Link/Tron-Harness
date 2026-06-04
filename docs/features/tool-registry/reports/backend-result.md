# Backend Result: Tool Registry

## Summary

- status: complete

## Evidence Rows

- AC-TOOL-01 | red: pre-evolution catalog responses exposed only business domains and lacked coding-domain schema coverage | green: `tron-backend/tests/agent_management_api.rs` now proves `/api/v1/tools/domains`, `/api/v1/tools`, and `/api/v1/tools/bash.exec` expose `bash`/`fs`/`search` plus top-level `description` and `examples` | command_ref: CMD-TOOL-BE-TEST | browser:not applicable | status: done.
- AC-TOOL-02 | red: expanding `all_platform_tools` risked regressing persisted internal-agent defaults and system-agent APIs | green: policy regression tests still pass and default selected-tool resolution now includes coding tools without changing system-agent behavior | command_ref: CMD-TOOL-BE-TEST | browser:not applicable | status: done.
- AC-TOOL-03 | red: main-agent direct-list runs could not materialize or execute bounded coding tools from policy-derived assignments | green: coding executors are registered, localized, and covered by `internal_agent_direct_list_materializes_and_executes_coding_tools`, including successful `bash.exec`, `fs.read_text`, and `search.glob` execution | command_ref: CMD-TOOL-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/application/agent/business_tools.rs`
- `tron-backend/src/application/agent/business_tools/discovery.rs`
- `tron-backend/src/application/agent/business_tools/coding_tools.rs`
- `tron-backend/src/application/agent/business_tools/coding_tools/bash.rs`
- `tron-backend/src/application/agent/business_tools/coding_tools/filesystem.rs`
- `tron-backend/src/application/agent/business_tools/coding_tools/search.rs`
- `tron-backend/src/application/agent/tool_catalog/definitions.rs`
- `tron-backend/src/application/agent/tool_catalog/definitions/business.rs`
- `tron-backend/src/application/agent/tool_catalog/definitions/coding.rs`
- `tron-backend/src/application/agent/tool_catalog/localization.rs`
- `tron-backend/src/application/agent/tool_catalog/localization/coding.rs`
- `tron-backend/tests/agent_management_api.rs`

## Commands

- `CMD-BE-CHECK`: passed.
- `CMD-TOOL-BE-TEST`: passed.

## Blockers

- None.
