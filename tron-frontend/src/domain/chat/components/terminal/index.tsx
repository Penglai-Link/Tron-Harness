import { initTerminal } from '@tron/chat-terminal';
import { useEffect, useMemo, useRef } from 'react'
import { parseTerminalMessage, TerminalSocketClient } from './socketClient';
import { explainResultMessage, generateAiChatMessage, generateCommandMessage } from './prompt';
import { fetchEventSource } from '@microsoft/fetch-event-source';
import "@xterm/xterm/css/xterm.css";
import "@tron/chat-terminal/styles.css"

const Terminal = () => {
    const terminalRef = useRef<HTMLDivElement>(null);
    const socketClient = new TerminalSocketClient(`http://${window.location.hostname}:8787/`);
    const terminalInstance = useMemo(() => {
        if (terminalRef.current) {
            return initTerminal({
                el: terminalRef.current,
                termOptions: {
                    themeTokens: {
                        // 终端背景色
                        background: "var(--color-header-backdrop, 'white')",
                        // 输入的文字颜色
                        foreground: "var(--color-on-surface, 'black')",
                        // 光标颜色
                        cursor: "var(--color-primary, red)",
                        // 图标颜色
                        icon: "var(--color-primary, red)",
                        subtle: "var(--color-on-surface, 'black')",

                        // AI对话面板
                        panelBackground: "var(--color-elevated-surface)",
                        muted: "var(--color-on-surface, 'black')",
                        border: "var(--color-card-border, 'gary')",
                        hoverBackground: "var(--color-surface-container-high, 'gary')",
                        inputBackground: "var(--color-header-backdrop, 'white')",

                        accent: "var(--success-foreground)",
                        success: "var(--success-foreground)",
                        // 扩展面板前景色
                        panelForeground: "var(--color-on-background, yellow)",
                        // 选中文本区域的背景色
                        selectionBackground: "var(--color-accent-container, red)",
                    },
                },
                runCommand: (command, onOutput) =>
                    socketClient.execute(command, onOutput),
                // ai对话
                commandChat: (command) => fetchEventSource("http://127.0.0.1:8787/api/llm/v1/chat/completions", {
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
                        terminalInstance?.appendChatChunk(delta, { requestId: messageId });
                    },
                    onclose() {
                        terminalInstance?.completeChat("")
                    },
                }),
                // 解释输出结果
                commandExplain: ({ command, output }, { signal }) =>
                    chat(explainResultMessage(command, output), signal),
                // 根据命令重新生成命令
                commandGenerate: (content) =>
                    chat(generateCommandMessage(content)),
            });
        }
    }, [terminalRef.current])

    function chat(content: string, signal?: AbortSignal) {
        return fetch("http://127.0.0.1:8787/api/llm/v1/chat/completions", {
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
    }


    function connect() {
        socketClient.connect().catch((error) => {
            console.warn(error);
        });
    }

    useEffect(() => {
        connect()
        return () => {
            window.addEventListener("beforeunload", () => socketClient.destroy(), {
                once: true,
            });
        }
    }, [])




    return (
        <div ref={terminalRef} className='w-full h-full relative'></div>
    )
}

export default Terminal