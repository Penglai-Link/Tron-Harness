import { AITransport, createTerminalTransport, initTerminal } from '@tron/chat-terminal';
import { useCallback, useEffect, useRef } from 'react'
import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from './prompt';
import "@xterm/xterm/css/xterm.css";
import "@tron/chat-terminal/styles.css"
import { useTheme } from '@/shared';
import { useLocale } from '@/i18n';
import { createInternalChatAdapter } from '@tron/chat-internal-adapter';

const Terminal = () => {
    const terminalRef = useRef<HTMLDivElement>(null);
    const terminalInstanceRef = useRef<ReturnType<typeof initTerminal> | null>(null);
    const { theme } = useTheme()
    const { currentLocale } = useLocale()

    const terminalServerUrl =
        import.meta.env.VITE_TERMINAL_SERVER_URL ||
        `http://${window.location.hostname}:8787`;


    const chatContextRef = useRef<ReturnType<typeof createChatContext> | null>(null);

    const chatAdapter = useCallback(() => {
        if (!chatContextRef.current) {
            const context = createChatContext();
            chatContextRef.current = context.catch((error) => {
                chatContextRef.current = null;
                throw error;
            });
        }
        return chatContextRef.current;
    }, [])

    useEffect(() => {
        const container = terminalRef.current;
        if (!container) {
            return;
        }

        const terminalTransport = createTerminalTransport({
            url: terminalServerUrl,
        });

        const aiTransport: AITransport = {
            async stream(content, { onChunk, signal }) {
                const requestId = crypto.randomUUID();
                const { adapter, session } = await chatAdapter()
                await adapter.streamMessage(session, {
                    content,
                    workspaceCode: "WS-EC0B2F07415C03F6"
                }, {
                    onDelta(delta) {
                        onChunk({ delta: delta, requestId });
                    },
                    
                }, signal)
                return { requestId };
            },

            async request(content, { signal } = {}) {
                const { adapter, session } = await chatAdapter();
                let response = "";
                await adapter.streamMessage(session, {
                    content,
                    workspaceCode: "WS-EC0B2F07415C03F6",
                }, {
                    onDelta(delta) {
                        response += delta;
                    },
                }, signal);
                return response;
            },
        };



        const terminalInstance = initTerminal({
            el: container,
            transport: {
                terminal: terminalTransport,
                ai: aiTransport,
            },
            prompts: {
                chat: generateAiChatMessage,
                explain: ({ command, output }) => explainResultMessage(command, output),
                generate: generateCommandMessage,
            },
            terminalOptions: {
                themeTokens: {
                    primary: "var(--color-on-surface, black)",
                    secondary: "var(--color-header-backdrop, white)",
                    cardBackground: "var(--color-elevated-surface, white)",
                    border: "var(--color-card-border, gray)",
                },
            },
        });
        terminalInstanceRef.current = terminalInstance;

        return () => {
            terminalInstance.dispose();
            if (terminalInstanceRef.current === terminalInstance) {
                terminalInstanceRef.current = null;
            }
        }
    }, [chatAdapter])


    useEffect(() => {
        terminalInstanceRef.current?.refreshTheme()
    }, [theme])

    useEffect(() => {
        terminalInstanceRef.current?.setLocale(currentLocale)
    }, [currentLocale])


    return (
        <div ref={terminalRef} className='w-full h-full relative py-2.5 pl-2.5'></div>
    )
}

export default Terminal

async function createChatContext() {
    const { accessToken } = await fetch("/api/v1/auth/refresh", { method: "POST" })
        .then((res) => res.json()).then(res => res.data)
    const adapter = createInternalChatAdapter({
        getAccessToken: () => accessToken,
    });
    const session = await adapter.createSession({ workspaceCode: "WS-EC0B2F07415C03F6" });
    return { adapter, session };
}
