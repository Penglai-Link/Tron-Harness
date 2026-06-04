# Feature Spec: Response API Parallel Tool Calls

## Summary

Evolve internal-agent runtime limits so the existing `maxToolCalls` policy remains the total tool-call budget for a run, while a new `maxParallelToolCalls` policy caps only the number of tool calls executed concurrently inside one model turn. When a model returns more tool calls than the parallel cap, runtime should execute them in batches instead of failing the run up front.

## Scope

- Backend runtime batching for internal-agent tool calls.
- Backend internal-agent policy contract for a separate parallel batch cap.
- Workspace `/settings/agents` internal-agent policy UI updates for the new field and clarified labels.

## Out Of Scope

- Provider-specific tool-choice routing changes.
- Multi-agent CLI runtime batching behavior.
- Conversation memory policy changes.

## Behavior

1. `maxToolCalls` remains the total tool-call budget across the whole run.
2. `maxParallelToolCalls` limits how many tool calls may run concurrently from a single tool-call response.
3. When a model returns more tool calls than `maxParallelToolCalls`, runtime executes them in sequential batches sized by the smaller of:
   - remaining total tool-call budget
   - configured parallel cap
   - remaining tool calls in the current response
4. If the run exhausts the total budget after executing the last allowed call, any additional pending call in the same response fails the run with the existing max-tool-calls error.
5. Workspace internal-agent policy UI must present both limits distinctly so operators do not confuse total budget with concurrency.

## Affected Surfaces

- Backend: React runtime tool-call execution, internal-agent policy persistence and APIs.
- Frontend: workspace internal-agent policy form, model normalization, mock data, translated help text.

## References

- @ref:docs/features/react-agent-runtime/spec/feature-spec.md
- @ref:docs/features/tool-registry/spec/feature-spec.md
