# Backend Task

## Task Rows

| row | phase | ac | scope | status | notes |
| --- | --- | --- | --- | --- | --- |
| 1 | tdd | AC-RPTC-01, AC-RPTC-02 | `tron-backend` internal-agent policy contract, persistence, and runtime batching semantics | completed | Added `maxParallelToolCalls`, SQLite migration, API roundtrip, sequential batch execution for parallel tool-call sets, and regression coverage for partial progress before total-budget failure. |
