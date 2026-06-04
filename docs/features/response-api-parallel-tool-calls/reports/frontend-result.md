# Frontend Result

- status: completed
- ac: `AC-RPTC-03`
- changed:
  - Added `maxParallelToolCalls` to internal-agent policy models, API normalization, and mocks.
  - Updated workspace internal-agent drafts and autosave payload generation to manage both total tool-call budget and parallel batch cap.
  - Clarified labels/help text so `maxToolCalls` reads as total budget and `maxParallelToolCalls` reads as concurrency cap.
- evidence:
  - `CMD-FE-CHECK`: passed
