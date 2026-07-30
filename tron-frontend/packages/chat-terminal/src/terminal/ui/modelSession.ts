import { html, render } from "lit-html";
import { guard } from "lit-html/directives/guard.js";
import type {
  ModelSessionCommandCardState,
  ModelSessionCommandToken,
  ModelSessionRenderActions,
  ModelSessionRenderState,
  ModelSessionToken,
} from "../types/modelSession";
import { renderMarkdown as renderMarkdownContent } from "../utils/markdown";

const COMMAND_STATUS_TEXT: Record<
  ModelSessionCommandCardState["status"],
  string
> = {
  pending: "待确认",
  editing: "修改中",
  executing: "执行中",
  executed: "已执行",
  rejected: "已拒绝",
  error: "失败",
};

export function createModelSessionHost() {
  const host = document.createElement("div");
  host.className = "ai-terminal-model-session-host";
  host.addEventListener("mousedown", (event) => {
    event.stopPropagation();
  });
  return host;
}

export function renderModelSession(
  host: HTMLElement,
  state: ModelSessionRenderState,
  actions: ModelSessionRenderActions,
) {
  render(
    html`
      <section class="ai-terminal-model-session">
        <div class="ai-terminal-model-session__body">
          ${state.tokens.map((token) =>
            renderModelSessionToken(token, state, actions),
          )}
        </div>
      </section>
    `,
    host,
  );
}

function renderModelSessionToken(
  token: ModelSessionToken,
  state: ModelSessionRenderState,
  actions: ModelSessionRenderActions,
) {
  if (token.type === "markdown") {
    return html`<div
      class="ai-terminal-model-session__markdown ai-terminal-markdown"
    >
      ${guard([token.content], () => renderMarkdownContent(token.content))}
    </div>`;
  }

  const cardState = state.commandStates.get(token.blockId) ?? {
    status: "pending" as const,
    command: token.command,
    draftCommand: token.command,
  };

  return renderCommandCard(token, cardState, actions);
}

function renderCommandCard(
  token: ModelSessionCommandToken,
  state: ModelSessionCommandCardState,
  actions: ModelSessionRenderActions,
) {
  const isGenerating = state.isGenerating === true;
  const isBusy = state.status === "executing" || isGenerating;
  const isTerminal = state.status === "executed" || state.status === "rejected";
  const canExecute =
    !isBusy && (state.status === "pending" || state.status === "error");
  const canEdit = !isBusy && !isTerminal;
  const canReject =
    !isBusy && state.status !== "executed" && state.status !== "rejected";

  return html`
    <article
      class=${`ai-terminal-model-session-command ai-terminal-model-session-command--${state.status}`}
    >
      <div class="ai-terminal-model-session-command__meta">
        <span class="ai-terminal-model-session-command__type">
          是否同意执行以下命令并查看输出？
        </span>
        ${state.status === "pending" || state.status === "editing"
          ? html`<div class="ai-terminal-model-session-command__actions">
              <button
                class="ai-terminal-model-session-command__button ai-terminal-model-session-command__button--primary"
                type="button"
                ?disabled=${!canExecute}
                @click=${() => actions.onExecuteCommand(token)}
              >
                执行
              </button>
              ${state.status === "editing"
                ? html`
                    <button
                      class="ai-terminal-model-session-command__button"
                      type="button"
                      ?disabled=${isBusy}
                      @click=${() => actions.onSaveCommandEdit(token)}
                    >
                      保存
                    </button>
                  `
                : html`
                    <button
                      class="ai-terminal-model-session-command__button"
                      type="button"
                      ?disabled=${!canEdit}
                      @click=${(event: MouseEvent) => {
                        const button = event.currentTarget as HTMLElement;
                        const commandDisplay = button
                          .closest<HTMLElement>(
                            ".ai-terminal-model-session-command",
                          )
                          ?.querySelector<HTMLElement>(
                            ".ai-terminal-model-session-command__code",
                          );

                        actions.onStartEditCommand(
                          token,
                          commandDisplay?.getBoundingClientRect().height,
                        );
                      }}
                    >
                      修改
                    </button>
                  `}
              <button
                class="ai-terminal-model-session-command__button"
                type="button"
                ?disabled=${!canReject}
                @click=${() => actions.onRejectCommand(token)}
              >
                拒绝
              </button>
            </div>`
          : state.status === "executed" || state.status === "rejected"
            ? html`<span class="ai-terminal-model-session-command__status">
                ${renderCommandStatus(state.status)}
              </span>`
            : null}
      </div>

      ${html`
            <div
              class="ai-terminal-model-session-command__editor"
              style=${
                state.editorHeight ? `height:${state.editorHeight}px;` : ""
              }
            >
              <input
                .readOnly=${state.status !== "editing" || isGenerating}
                .value=${state.status === "editing" ? state.draftCommand : state.command}
                @input=${(event: Event) => {
                  const target = event.currentTarget as HTMLInputElement;
                  actions.onDraftCommandChange(token, target.value);
                }}
                spellcheck="false"
              ></input>
              ${
                state.status === "editing" && actions.onGenerateCommand
                  ? html`
                      <button
                        class="ai-terminal-model-session-command__ai-generate"
                        type="button"
                        ?disabled=${isGenerating || !state.draftCommand.trim()}
                        @click=${() => actions.onGenerateCommand?.(token)}
                      >
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          width="24"
                          height="24"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          class="lucide lucide-pencil-sparkles-icon lucide-pencil-sparkles"
                        >
                          <path d="M10 3H8" />
                          <path d="m15.007 5.008 3.987 3.986" />
                          <path d="M20 15v4" />
                          <path
                            d="M21.174 6.813a2.82 2.82 0 0 0-3.986-3.987L3.842 16.175a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"
                          />
                          <path d="M22 17h-4" />
                          <path d="M4 5v4" />
                          <path d="M6 7H2" />
                          <path d="M9 2v2" />
                        </svg>
                        ${isGenerating ? "生成中" : "AI生成"}
                      </button>
                    `
                  : ""
              }
            </div>
          `} ${state.error
        ? html`<div class="ai-terminal-model-session-command__error">
            ${state.error}
          </div>`
        : null}
    </article>
  `;
}

function renderCommandStatus(status: ModelSessionCommandCardState["status"]) {
  return COMMAND_STATUS_TEXT[status];
}
