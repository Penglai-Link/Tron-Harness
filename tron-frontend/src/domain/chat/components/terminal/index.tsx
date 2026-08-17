import { createAITransport, createTerminalTransport, initTerminal } from '@tron/chat-terminal';
import { useEffect, useRef } from 'react'
import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from './prompt';
import "@xterm/xterm/css/xterm.css";
import "@tron/chat-terminal/styles.css"
import { useTheme } from '@/shared';
import { useLocale } from '@/i18n';

const Terminal = () => {
    const terminalRef = useRef<HTMLDivElement>(null);
    const terminalInstanceRef = useRef<ReturnType<typeof initTerminal> | null>(null);
    const { theme } = useTheme()
    const { currentLocale } = useLocale()

    const terminalServerUrl =
        import.meta.env.VITE_TERMINAL_SERVER_URL ||
        `http://${window.location.hostname}:8787`;

    useEffect(() => {
        const container = terminalRef.current;
        if (!container) {
            return;
        }

        const terminalTransport = createTerminalTransport({
            url: terminalServerUrl,
        });
        const aiTransport = createAITransport({
            url: `${terminalServerUrl}/api/llm/v1/chat/completions`,
        });

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
    }, [])

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
