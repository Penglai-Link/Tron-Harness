import { initTerminal } from '@tron/chat-terminal';
import { useEffect, useRef } from 'react'
import { parseTerminalMessage, TerminalSocketClient } from './socketClient';
import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from './prompt';
import { fetchEventSource } from '@microsoft/fetch-event-source';
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

    const chat = (content: string, signal?: AbortSignal) =>
        fetch(`${terminalServerUrl}/api/llm/v1/chat/completions`, {
            method: "POST",
            body: JSON.stringify({
                messages: [{ content, role: "user" }],
                stream: false,
            }),
            headers: {
                "Content-Type": "application/json",
                Authorization: "Bearer sk-42f0b5eb8a0043509c6c48270314c90d",
            },
            signal,
        }).then((res) => res.json()).then((payload) => parseTerminalMessage(payload)?.delta || '')

    useEffect(() => {
        const container = terminalRef.current;
        if (!container) {
            return;
        }

        const socketClient = new TerminalSocketClient(
            terminalServerUrl,
            {
                onSessionChange: (currentDirectory) => {
                    terminalInstanceRef.current?.setCurrentDirectory(currentDirectory);
                },
            },
        );
        const terminalInstance = initTerminal({
            el: container,
            termOptions: {
                themeTokens: {
                    primary: "var(--color-on-surface, black)",
                    secondary: "var(--color-header-backdrop, white)",
                    cardBackground: "var(--color-elevated-surface, white)",
                    border: "var(--color-card-border, gray)",
                },
            },
            runCommand: (command, onOutput, { interactive }) =>
                socketClient.execute(command, onOutput, { interactive }),
            cancelCommand: () => socketClient.cancelCurrent(),
            resizeTerminal: (size) => socketClient.resize(size),
            writeCommandInput: (data) => socketClient.sendInput(data),
            commandChat: (command) => fetchEventSource(`${terminalServerUrl}/api/llm/v1/chat/completions`, {
                method: "POST",
                body: JSON.stringify({
                    messages: [{ content: generateAiChatMessage(command), role: "user" }],
                    stream: true,
                }),
                headers: {
                    "Content-Type": "application/json",
                    'Accept': 'text/event-stream',
                    Authorization: "Bearer sk-42f0b5eb8a0043509c6c48270314c90d",
                },
                onmessage({ data }) {
                    const { messageId, delta } = JSON.parse(data)
                    terminalInstanceRef.current?.appendChatChunk(delta, { requestId: messageId });
                },
                onclose() {
                    terminalInstanceRef.current?.completeChat("")
                },
            }),
            commandExplain: ({ command, output }, { signal }) =>
                chat(explainResultMessage(command, output), signal),
            commandGenerate: (content) =>
                chat(generateCommandMessage(content)),
        });
        terminalInstanceRef.current = terminalInstance;


        socketClient.connect().catch((error) => {
            console.warn(error);
        });

        return () => {
            terminalInstance.dispose();
            if (terminalInstanceRef.current === terminalInstance) {
                terminalInstanceRef.current = null;
            }
            socketClient.destroy();
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
