import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { api, bootstrap, list, object, snapshot, string, type ObjectValue, type Snapshot } from './api';
import { choiceLabel, messages } from './conversation';
import { ManifestView } from './manifest-view';
import './style.css';
function App() {
    const [state, setState] = useState<Snapshot>();
    const [selected, setSelected] = useState('');
    const [text, setText] = useState('');
    const [error, setError] = useState('');
    const [busy, setBusy] = useState(false);
    const [tab, setTab] = useState('Plano');
    const [candidate, setCandidate] = useState<ObjectValue>();
    const [featureId, setFeatureId] = useState('');
    const [title, setTitle] = useState('');
    const [request, setRequest] = useState('');
    const [preview, setPreview] = useState<ObjectValue>();
    const actionBusy = useRef(false);
    const end = useRef<HTMLDivElement>(null);
    const input = useRef<HTMLTextAreaElement>(null);
    useEffect(() => { let alive = true; const abort = new AbortController(); let timer: ReturnType<typeof setTimeout>; const poll = async () => { try {
        const s = snapshot(await api('state', undefined, abort.signal));
        if (alive) {
            setState(s);
            setSelected(old => old || s.sessions[0]?.id || '');
        }
    }
    catch (e) {
        if (alive)
            setError(e instanceof Error ? e.message : 'Conexão perdida');
    }
    finally {
        if (alive)
            timer = setTimeout(() => void poll(), 450);
    } }; void bootstrap().then(poll).catch(e => { if (alive)
        setError(String(e.message)); }); return () => { alive = false; clearTimeout(timer); abort.abort(); }; }, []);
    const session = state?.sessions.find(s => s.id === selected);
    const entries = messages(session?.events ?? []);
    useEffect(() => { end.current?.scrollIntoView({ block: 'nearest' }); }, [session?.events.length, entries.at(-1)?.text]);
    const action = async (route: string, body: ObjectValue = {}) => { if (actionBusy.current)
        return; actionBusy.current = true; setBusy(true); setError(''); try {
        return object(await api(route, body));
    }
    catch (e) {
        setError(e instanceof Error ? e.message : 'Ação falhou; resultado desconhecido.');
        return undefined;
    }
    finally {
        actionBusy.current = false;
        setBusy(false);
    } };
    const newSession = async () => { const r = await action('sessions'); if (r) {
        setSelected(string(r.thread_id));
        setText('');
        input.current?.focus();
    } };
    const send = async () => { if (!text.trim())
        return; let id = selected; if (!id) {
        const r = await action('sessions');
        if (!r)
            return;
        id = string(r.thread_id);
        setSelected(id);
    } const r = await action('turn', { thread_id: id, text }); if (r)
        setText(''); };
    const connected = state?.connected === true;
    const ready = connected && state?.authenticated === true && (!session || session.loaded);
    const usage = object(session?.usage.total);
    const last = object(session?.usage.last);
    const duration = object(candidate?.duration_estimate);
    const range = object(duration.remaining_seconds);
    return <><a className="skip" href="#conversation">Ir à conversa</a><div className="workbench">
 <aside className="sidebar" aria-label="Projeto e sessões"><a className="brand" href="/" aria-label="Illumitech Forge"><img src="/logo.webp" alt="Illumitech"/><span>FORGE<small>WORKBENCH · LOCAL</small></span></a><div className="project"><span className="eyebrow">PROJETO AUTORIZADO</span><strong><span className="project-icon">⌘</span>{state?.project || 'Conectando…'}</strong><small>Sessões deste projeto · permissões nativas</small></div><button className="primary" data-testid="new-session" disabled={!connected || busy} onClick={() => void newSession()}>＋ Nova conversa</button><div className="section-label">SESSÕES <span>{state?.sessions.length ?? 0}</span></div><nav data-testid="session-list" className="sessions" aria-label="Sessões deste projeto">{state?.sessions.map(s => <button key={s.id} aria-current={s.id === selected ? 'page' : undefined} onClick={() => { setSelected(s.id); void action('resume', { thread_id: s.id }); }} disabled={busy || !connected}><span>◌</span><span>{s.title || s.id}<small>{s.active_turn ? 'Em andamento' : 'Conversa nativa'}</small></span></button>)}{!state?.sessions.length && <p className="muted">Suas conversas neste projeto aparecerão aqui.</p>}</nav><div className="sidebar-bottom"><span className="local-dot"/> Ambiente local<small>Conta e configuração gerenciadas pelo cliente nativo.</small></div></aside>
 <main id="conversation" className="main"><header className="topbar"><div><span className="eyebrow">ILLUMITECH / ESPAÇO DE TRABALHO</span><h1>{session?.title || 'Uma ideia. Um plano. Evidências.'}</h1></div><span role="status" data-testid="connection-state" className={`connection ${connected ? 'online' : ''}`}>{state ? (connected ? 'Conectado ao nativo' : 'Desconectado') : 'Conectando…'}</span></header><div className="modebar"><span className="mode">◉ Conversa livre</span><span>Read-only · aprovação nativa sob demanda</span><button onClick={() => void action('reconnect')} disabled={busy}>Reconectar</button></div>
 {(error || state?.error) && <div role="alert" className="error">{error || state?.error}<small>Nenhuma ação será reenviada automaticamente.</small><button onClick={() => setError('')}>Dispensar aviso</button></div>}
 {session && !session.loaded && connected && <div className="notice">Sessão salva; retome explicitamente antes de enviar.<button onClick={() => void action('resume', { thread_id: selected })} disabled={busy}>Retomar sessão</button></div>}
 {state && !state.authenticated && <div className="error" role="alert">Login gerenciado necessário. Entre pelo cliente Codex nativo e reconecte. Credenciais não são solicitadas nesta interface.</div>}
 <section className="conversation" data-testid="conversation" aria-label="Conversa" aria-live="polite" aria-relevant="additions text">{!entries.length && <div className="empty"><div className="forge-glyph" aria-hidden="true">F<span>↗</span></div><span className="eyebrow">DO CONTEXTO À CONSTRUÇÃO</span><h2>O próximo passo começa<br />com uma boa conversa.</h2><p>Explore uma ideia, revise uma mudança ou prepare um plano.<br />Evidências e decisões ficam sempre à vista.</p><div className="empty-cards"><button disabled={!ready} onClick={() => { setText('Analise as convenções deste projeto em modo somente leitura.'); input.current?.focus(); }}>⌘ Entender o projeto<small>Contexto antes de implementar</small></button><button onClick={() => setTab('Criar')}>↗ Preparar uma feature<small>Preview antes de escrever</small></button></div></div>}{entries.map(m => <article key={m.id} className={`message ${m.kind}`}><div className="message-label">{m.kind === 'userMessage' ? 'VOCÊ' : m.kind === 'agentMessage' ? 'CODEX / NATIVO' : m.kind === 'decision' ? 'DECISÃO NATIVA' : m.kind === 'turn' ? 'ESTADO DO TURNO' : m.kind.toUpperCase()} {m.status && <span>{m.status}</span>}</div>{['commandExecution', 'fileChange', 'diff', 'plan'].includes(m.kind) ? <details open={m.kind === 'diff' || m.kind === 'fileChange'}><summary>{m.kind === 'fileChange' ? 'Mudanças em arquivos' : m.kind === 'diff' ? 'Diff do turno' : m.kind === 'plan' ? 'Plano proposto pelo agente' : 'Execução de ferramenta'}</summary><pre>{m.text || 'Aguardando saída…'}</pre></details> : <p>{m.text || 'Pensando…'}</p>}</article>)}
 {state?.approvals.filter(a => a.params.threadId === selected).map(a => <article className="approval" key={JSON.stringify(a.id)}><span className="eyebrow">APROVAÇÃO NATIVA PENDENTE</span><h3>{string(a.params.reason) || 'O cliente nativo solicita uma decisão'}</h3><pre>{string(a.params.command) || JSON.stringify(a.params, null, 2)}</pre><p>Revise a ação e o escopo. Esta decisão não aprova uma revisão Forge.</p><div className="approval-actions">{a.choices.map((c, i) => <button key={i} disabled={busy || !connected} onClick={() => void action('approval', { thread_id: selected, request_id: a.id, decision: c })}>{choiceLabel(c)}</button>)}{!a.choices.length && <p role="alert">Solicitação nativa não suportada por esta versão. Bloqueada; nenhuma permissão concedida.</p>}</div></article>)}<div ref={end}/></section>
 <form className="composer" onSubmit={e => { e.preventDefault(); void send(); }}><label className="sr-only" htmlFor="chat">Mensagem para o cliente nativo</label><textarea id="chat" ref={input} data-testid="chat-input" placeholder="Descreva sua ideia ou pergunte sobre o projeto…" value={text} disabled={!ready || busy} onChange={e => setText(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        void send();
    } }}/><div className="composer-controls"><span>{string(session?.effective.model) || 'Modelo da configuração nativa'}<small>⌘ / Ctrl + Enter para enviar</small></span>{session?.active_turn ? <button type="button" data-testid="interrupt-turn" onClick={() => void action('interrupt', { thread_id: selected })} disabled={busy || !connected}>■ Interromper turno</button> : <button className="primary" data-testid="send-message" disabled={!ready || busy || !text.trim()} type="submit">{busy ? 'Enviando…' : 'Enviar ↑'}</button>}</div></form><footer className="chat-footer">Interromper turno não garante encerrar terminais nativos em segundo plano. Chat não é evidência de conclusão. Gates e aceite de revisão pertencem ao Forge.</footer></main>
 <aside className="details" aria-label="Plano e evidências"><div className="details-heading"><span className="eyebrow">CONTEXTO DO TRABALHO</span><h2>Da intenção à evidência</h2></div><div className="tabs" role="tablist" aria-label="Detalhes">{['Plano', 'Evidências', 'Consumo', 'Criar'].map(t => <button id={`tab-${t}`} key={t} role="tab" aria-selected={tab === t} aria-controls="detail-panel" tabIndex={tab === t ? 0 : -1} onKeyDown={e => { if (['ArrowRight', 'ArrowLeft', 'Home', 'End'].includes(e.key)) {
        e.preventDefault();
        const tabs = ['Plano', 'Evidências', 'Consumo', 'Criar'];
        const next = e.key === 'Home' ? 0 : e.key === 'End' ? 3 : (tabs.indexOf(t) + (e.key === 'ArrowRight' ? 1 : 3)) % 4;
        setTab(tabs[next]);
        document.getElementById(`tab-${tabs[next]}`)?.focus();
    } }} onClick={() => setTab(t)}>{t}</button>)}</div><section role="tabpanel" id="detail-panel" aria-labelledby={`tab-${tab}`}>
 {tab === 'Plano' && <><span className="badge">PROPOSTA ≠ EXECUÇÃO</span><h3>Planeje antes de agir</h3><p>Pedido → specs → dependências → comandos → previsão. O planejamento nativo é somente leitura.</p><label htmlFor="request">O que você quer construir?</label><textarea id="request" value={request} onChange={e => setRequest(e.target.value)} placeholder="Objetivo e resultado observável"/><button disabled={!request.trim() || busy} onClick={async () => { const r = await action('planning-preview', { request }); if (r)
        setCandidate(r); }}>Prévia sem inferência</button><button className="primary" disabled={!ready || busy || !request.trim()} onClick={async () => { const r = await action('planning-propose', { request }); if (r)
        setCandidate(r); }}>{busy ? 'Preparando…' : 'Propor com cliente nativo'}</button>{candidate && <><h3>Prévia de execução</h3>{list(candidate.order).map(id => <p key={String(id)}>{String(id)}</p>)}<p>Duração: {typeof range.lower === 'number' ? `${Math.round(range.lower)}–${Math.round(Number(range.upper))} s` : 'indisponível até validar o DAG'}</p><details><summary>Contrato, estimativas e comandos</summary><pre>{JSON.stringify(candidate, null, 2)}</pre></details></>}<div className="notice">Nenhum plano executado. A proposta precisa de revisão, comandos e gates externos fixados.</div></>}
 {tab === 'Evidências' && <><span className="badge">CONTROLES LOCAIS</span><h3>Aceite por revisão</h3><p>{string(state?.forge.reason) || 'Carregando estado operacional…'}</p><p>Uma resposta do agente e uma aprovação de ferramenta não significam que um gate passou.</p><ManifestView onLoad={setCandidate} connected={ready}/></>}
 {tab === 'Consumo' && <><span className="badge">MEDIÇÃO NATIVA</span><h3>Tokens observados</h3><dl>{[['Entrada', 'inputTokens'], ['Cache incluído', 'cachedInputTokens'], ['Saída', 'outputTokens'], ['Total', 'totalTokens']].map(([label, key]) => <div key={key}><dt>{label}</dt><dd>{typeof usage[key] === 'number' ? usage[key].toLocaleString('pt-BR') : '—'}</dd></div>)}</dl><p>{Object.keys(usage).length ? 'Acumulado reportado pelo cliente para esta sessão.' : 'Uso ainda não observado; ausência não significa zero.'}</p><details><summary>Último turno e cobertura</summary><pre>{JSON.stringify(last, null, 2)}</pre></details><div className="notice">Estimativas de contexto não são tokens medidos, saldo da assinatura ou custo monetário.</div></>}
 {tab === 'Criar' && <><span className="badge">CRIAÇÃO GUIADA</span><h3>Prepare uma feature</h3><label htmlFor="feature-id">ID permanente</label><input id="feature-id" placeholder="001-minha-feature" value={featureId} onChange={e => { setFeatureId(e.target.value); setPreview(undefined); }}/><label htmlFor="feature-title">Título</label><input id="feature-title" value={title} onChange={e => { setTitle(e.target.value); setPreview(undefined); }}/><button disabled={busy || !featureId || !title} onClick={async () => { const r = await action('feature-preview', { id: featureId, title }); if (r)
        setPreview(r); }}>Revisar arquivos</button>{preview && <><p>spec.md, plan.md, tasks.md e manifest externo. Nenhum instalador ou executor é iniciado.</p><details open><summary>Conteúdo a criar</summary><pre>{JSON.stringify(preview.files, null, 2)}</pre></details><button className="primary" disabled={busy} onClick={async () => { const r = await action('feature-apply', { id: featureId, title }); if (r)
        setPreview(r); }}>Criar arquivos revisados</button><p role="status">{preview.status === 'created' ? 'Arquivos criados. Aceite pendente.' : 'Prévia; nenhum arquivo escrito.'}</p></>}</>}
 </section><div className="details-footer"><span>◈ Evidências primeiro.</span><small>Illumitech Forge · 0.6 workbench</small></div></aside></div></>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>);
