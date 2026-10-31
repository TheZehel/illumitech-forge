import { Children, isValidElement, memo, useEffect, useRef, useState, type ReactNode } from 'react';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { choiceLabel, type Message } from './conversation';

function plainText(node: ReactNode): string {
    return Children.toArray(node).map(child => typeof child === 'string' || typeof child === 'number'
        ? String(child) : isValidElement<{ children?: ReactNode }>(child) ? plainText(child.props.children) : '').join('');
}
function CodeBlock({ children }: { children?: ReactNode }) {
    const [copied, setCopied] = useState(false);
    const [failed, setFailed] = useState(false);
    const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
    useEffect(() => () => clearTimeout(timer.current), []);
    return <div className="code-block"><div className="code-toolbar"><span>Código</span><button type="button" onClick={() => {
        void navigator.clipboard.writeText(plainText(children).replace(/\n$/, '')).then(() => {
            setCopied(true); setFailed(false); clearTimeout(timer.current);
            timer.current = setTimeout(() => setCopied(false), 2000);
        }).catch(() => setFailed(true));
    }}>{copied ? 'Copiado' : 'Copiar código'}</button><span className="sr-only" role="status">{copied ? 'Código copiado.' : failed ? 'Não foi possível copiar. Selecione o código para copiar.' : ''}</span></div><pre>{children}</pre></div>;
}
// React AST only. Raw HTML is skipped and the library's safe URL transform is preserved.
// Images require an explicit click; model output cannot issue network requests itself.
const markdownComponents = {
    pre: CodeBlock,
    a: ({ href, children }: { href?: string; children?: ReactNode }) => href
        ? <a href={href} target="_blank" rel="noopener noreferrer">{children}</a> : <span>{children}</span>,
    img: ({ src, alt }: { src?: string; alt?: string }) => src
        ? <a className="image-link" href={src} target="_blank" rel="noopener noreferrer">Ver imagem: {alt || 'imagem sugerida'}</a>
        : <span>{alt || 'Imagem indisponível'}</span>,
};
const plugins = [remarkGfm];
const statusLabels: Record<string, string> = {
    streaming: 'Respondendo', inProgress: 'Em andamento', completed: 'Concluído', failed: 'Falhou',
    interrupted: 'Interrompido', declined: 'Recusado', blocked: 'Bloqueado', proposed: 'Proposta', submitted: 'Enviada',
};
const activityLabels: Record<string, string> = {
    commandExecution: 'Execução de ferramenta', fileChange: 'Mudanças em arquivos', diff: 'Alterações propostas',
    plan: 'Plano proposto', reasoning: 'Raciocínio disponibilizado pelo cliente', webSearch: 'Pesquisa',
};
export const MessageView = memo(function MessageView({ kind, text, status }: Omit<Message, 'id'>) {
    if (kind === 'turn') {
        const label = status === 'completed' ? 'Resposta concluída' : status === 'interrupted'
            ? 'Resposta interrompida' : status === 'failed' ? 'Não foi possível concluir a resposta' : 'Resposta encerrada';
        return <article className={`message turn ${status}`}><p>{label}</p>{text.includes(': ') && <details><summary>Detalhes do cliente</summary><pre>{text.slice(text.indexOf(': ') + 2)}</pre></details>}</article>;
    }
    if (kind === 'decision') return <article className="message decision"><p>Decisão enviada: {choiceLabel(text)}.</p></article>;
    const conversational = kind === 'agentMessage' || kind === 'userMessage';
    const label = kind === 'userMessage' ? 'Você' : kind === 'agentMessage' ? 'Forge' : kind === 'error' ? 'Atenção' : 'Atividade';
    return <article className={`message ${kind}`}>
        <div className="message-label">{label}{status && <span>{statusLabels[status] || 'Estado informado pelo cliente'}</span>}</div>
        {kind === 'agentMessage' ? <div className="markdown"><Markdown remarkPlugins={plugins} components={markdownComponents} skipHtml>{text}</Markdown></div>
            : conversational || kind === 'error' ? <p>{text}</p>
                : <details><summary>{activityLabels[kind] || 'Detalhes da atividade'}</summary><pre>{text || 'Sem saída informada.'}</pre></details>}
    </article>;
});

export function ResponseProgress({ label }: { label: string }) {
    return <div className="response-progress" role="status" data-testid="response-progress"><span className="thinking-dots" aria-hidden="true"><i/><i/><i/></span><span>{label}</span></div>;
}
export function ConversationSkeleton({ label }: { label: string }) {
    return <div className="conversation-skeleton" role="status" data-testid="conversation-skeleton"><span className="sr-only">{label}</span><div aria-hidden="true"><div className="skeleton skeleton-avatar"/><div className="skeleton skeleton-line"/><div className="skeleton skeleton-line"/><div className="skeleton skeleton-line short"/></div></div>;
}
export function SessionSkeleton() {
    return <div className="session-skeleton" aria-hidden="true">{[0, 1, 2].map(id => <div className="skeleton skeleton-line" key={id}/>)}</div>;
}
