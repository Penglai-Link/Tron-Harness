# Feature Spec: Chat AG-UI User Cards

## Initial Agent Draft

- drafted_from_requirement: yes
- drafted_from_repository_findings: yes
- best_practice_defaults_applied: yes
- user_confirmation_status: approved_with_defaults

## Feature Identity And Context

- feature_slug: chat-agui-user-cards
- feature_name: Chat AG-UI User Cards
- status: approved_with_defaults
- owner: clouds3n
- source_request: Evolve Chat Management so low-level AG-UI cards remain available only for debug, while user-facing reasoning and tool-call cards summarize live agent work in the transcript.
- main_agent: Codex lead agent
- dependency_features: `right-context-sidebar`, chat streaming, AG-UI debug preferences

## Requirement

Chat Management currently renders one AG-UI card per low-level stream event. That representation is useful for debugging, but it is too granular for normal users. The page should keep the low-level event cards as a debug-only surface with a new `off` setting, and add a user-facing work rail that summarizes reasoning and tool activity before the assistant answer.

User-facing reasoning cards appear when a run receives a reasoning start event. They default collapsed, show an animated "Thinking" summary while active, and expand to stream `REASONING_MESSAGE_CONTENT` for that reasoning segment only. If the same run emits another `REASONING_START` after the prior segment ends, the transcript must render a new Thinking card instead of appending to the earlier one. After a reasoning segment finishes, the collapsed card should keep a short truncated summary preview so operators can skim the result without reopening it. When tool calls happen after a reasoning segment and before the next reasoning segment starts, those tool steps belong to the earlier Thinking card, and the Thinking summary must include the segment duration plus the number of child tools.

User-facing tool entries appear when a run receives a tool-call start event. They summarize the action and target as compact steps inside the current Thinking card, expand to show tool arguments while running, and show matching tool activity logs only when the activity belongs to the same tool call. After the tool finishes, the expanded step shows the tool result instead of live logs. The assistant answer remains the main output card and should not visually compete with the work rail.

## Product Source Slices

- approved_brainstorm: source request, accepted with defaults in this workspace
- approved_prd: source request, accepted with defaults in this workspace
- selected_feature: chat-management user-facing AG-UI cards

## Scope Boundary

This feature touches Chat Management presentation, AG-UI debug preference settings, chat stream mock data, and the backend AG-UI protocol event conversion needed to expose tool arguments/results and stable tool-call ids.

## Assumptions

- Existing low-level AG-UI event cards remain useful for operators and should not be deleted.
- Debug `off` means "do not render low-level debug cards"; it does not disable user-facing reasoning/tool cards.
- User-facing cards are live transcript UI only; persisted historical raw AG-UI replay remains unchanged unless the stream supplies compatible events.
- Frontend cannot rely on reasoning `messageId` as a stable per-segment key because current streams may reuse the same reasoning message id across multiple reasoning cycles in one run.
- Activity logs must be matched by `toolCallId`; run-level activity without a tool id is not shown inside a tool card to avoid attribution errors.
- Tool summaries can be inferred from tool name and common argument fields such as command, cmd, file, path, or target.
- Frontend can attribute a tool card to the latest reasoning segment in the same run when the stream does not provide an explicit reasoning-parent id.

## In Scope

- Add `off` to AG-UI debug event card visibility settings.
- Render user-facing reasoning cards grouped by reasoning segment boundaries inside the latest run.
- Render user-facing tool cards grouped by run and tool call id, nesting them under the latest reasoning segment when one exists.
- Parse streamed tool arguments and results from AG-UI protocol events.
- Match activity deltas to tool cards only by matching tool-call id.
- Update mock stream events so local development demonstrates the new behavior.
- Add focused automated proof for debug off and grouped user cards.

## Out Of Scope

- New chat persistence schema.
- Backend execution log streaming beyond existing AG-UI protocol events.
- Full redesign of chat bubbles, session sidebar, or right context sidebar beyond the work-rail presentation.
- Internationalization expansion beyond current frontend resource coverage.

## Page And Module Surface

- `tron-frontend/src/domain/chat/pages/ChatManagementPage.tsx`
- `tron-frontend/src/domain/chat/hooks/useChatMessageListBySession.ts`
- `tron-frontend/src/domain/chat/model/ChatModel.ts`
- `tron-frontend/src/domain/settings/model/aguiDebugPreferences.ts`
- `tron-frontend/src/domain/settings/pages/SettingsGeneralPage.tsx`
- `tron-backend/src/application/agent/runtime.rs`
- `tron-backend/src/application/agent/agui_stream.rs`

## Repository Findings

- `ChatManagementPage` currently renders `AguiEventRail` by filtering live `ChatAguiStreamEvent` items through `shouldShowAguiEventType`.
- `aguiDebugPreferences` stores mode and custom event types in local storage and already controls Settings > General.
- Live chat streaming calls `onAguiEvent` for SSE protocol events and accumulates them in `liveAguiEvents`.
- Current user-facing reasoning aggregation keys only by `runCode`, so multiple reasoning loops inside the same run collapse into one card.
- Backend AG-UI protocol conversion currently emits tool start/end/result events but does not emit tool arguments and uses run plus tool name as the tool-call id.
- Current activity delta events do not carry a tool-call id, so frontend grouping must ignore unmatched activity until the backend includes a stable id.

## Open Decisions

- None.
