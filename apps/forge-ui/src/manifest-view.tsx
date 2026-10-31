import { useEffect, useRef, useState } from 'react';
import { api, list, object, string, type ObjectValue } from './api';
export function ManifestView({ onLoad, connected = false }: { onLoad: (value: ObjectValue) => void; connected?: boolean }) {
    const [value, setValue] = useState<ObjectValue>();
    const [error, setError] = useState('');
    const [readError, setReadError] = useState('');
    const [busy, setBusy] = useState(false);
    const [reviewer, setReviewer] = useState('');
    const [reason, setReason] = useState('');
    const [reviewed, setReviewed] = useState('');
    const [result, setResult] = useState('');
    const [reviewPlan, setReviewPlan] = useState<ObjectValue>();
    const [authorizedScope, setAuthorizedScope] = useState(false);
    const inFlight = useRef(false);
    const generation = useRef(0);
    useEffect(() => {
        let alive = true; const controller = new AbortController(); let timer: ReturnType<typeof setTimeout>;
        const poll = async () => {
            const requestedGeneration = generation.current;
            try {
                const next = object(await api('forge', undefined, controller.signal));
                if (alive && requestedGeneration === generation.current) { setValue(next); if (next.control_revision) setAuthorizedScope(true); setReadError(''); if (next.inspection) onLoad(object(next.inspection)); }
            } catch (e: unknown) { if (alive && requestedGeneration === generation.current) { setReadError(e instanceof Error ? e.message : 'Estado operacional indisponível'); setValue(undefined); setReviewed(''); } }
            finally { if (alive) timer = setTimeout(() => void poll(), 750); }
        };
        void poll();
        return () => { alive = false; clearTimeout(timer); controller.abort(); };
    }, [onLoad]);
    const revision = string(value?.control_revision);
    const state = object(value?.operational_state);
    const run = object(state.run);
    const status = string(run.status) || string(state.status);
    const review = object(value?.review);
    const inspection = object(value?.inspection);
    const trace = object(inspection.traceability);
    const issue = object(trace.issue);
    const publication = object(inspection.publication_preview);
    const published = object(trace.publication_journal);
    const sourceUrl = string(issue.url);
    const budget = object(value?.budget);
    const tasks = object(run.tasks);
    const act = async (route: string, fields: ObjectValue = {}) => {
        if (inFlight.current) return;
        inFlight.current = true; generation.current += 1; setBusy(true); setError(''); setResult('');
        try { const r = object(await api(route, { revision, ...fields })); if (r.review_plan) setReviewPlan(object(r.review_plan)); if (r.asynchronous === true) setValue(old => old ? { ...old, operation_active: true } : old); setResult(r.approved === true ? 'Aprovação registrada para esta revisão. Retome explicitamente.' : r.approved === false ? 'Revisão recusada. Nenhuma aprovação concedida.' : 'Pedido registrado. Acompanhe o estado e as evidências.'); setReviewed(''); }
        catch (e: unknown) { setError(e instanceof Error ? e.message : 'Resultado desconhecido; ação não será repetida.'); }
        finally { generation.current += 1; inFlight.current = false; setBusy(false); }
    };
    const permitted = value?.execution_allowed !== false && connected && !!revision && !busy && value?.operation_active !== true;
    const cacheEvents = list(state.events).filter(e => object(e).kind === 'gate_cache');
    return <section aria-label="Controles Forge">
        {(error || readError) && <p role="alert">{error || readError}</p>}
        {value && !revision && <p>{string(value.reason)}</p>}
        {authorizedScope && <div className="approval-actions" aria-label="Controles globais">
            <button disabled={busy} onClick={() => void act('forge-pause')}>Pausar execução</button>
            <button disabled={busy} onClick={() => void act('forge-cancel')}>Cancelar execução</button>
        </div>}
        {revision && <>
            <details open><summary>Issue, spec, tarefas e publicação</summary>
                {sourceUrl ? <>
                    <p><a href={sourceUrl} target="_blank" rel="noreferrer">{string(issue.repository)} #{String(issue.number)}</a></p>
                    <p>Origem: {issue.provenance === 'offline_snapshot' ? 'arquivo offline; sem atestado do GitHub' : 'consulta autenticada pelo gh'}.</p>
                    <p>Spec: <code>{string(issue.spec_id)}</code></p>
                    <p>Tarefas: {list(trace.tasks).map(String).join(', ')}</p>
                </> : <p>Nenhuma issue selecionada neste manifest.</p>}
                <p>Demanda remota não concede permissões. Revisão independente: {string(object(trace.independent_review).status) || 'desconhecida'}.</p>
                <p>{published.readback === 'verified' ? <>PR verificado: <a href={string(published.url)} target="_blank" rel="noreferrer">{string(published.url)}</a></> : 'Nenhum PR verificado no journal.'}</p>
                <details><summary>Proveniência, revisão e evidências</summary><pre data-testid="issue-traceability">{JSON.stringify(trace, null, 2)}</pre></details>
                <h4>Prévia local de publicação</h4>
                {publication.repository ? <>
                    <p>{string(publication.repository)} · {string(publication.remote)} · {string(publication.branch)} → {string(publication.base_branch)}</p>
                    <p>{publication.remote_write_allowed === true ? 'CI autorizada no contrato; quota do provedor não verificada.' : 'Escritas remotas bloqueadas pela política de CI.'}</p>
                    <p>{publication.evidence_ready === true ? 'Evidências atuais disponíveis.' : 'Evidências e revisão humana ainda necessárias.'}</p>
                </> : <p>Publicação não autorizada neste manifest.</p>}
                <p>Merge e release exigem ações separadas.</p>
                <details><summary>Identidade, comandos e permissões</summary><pre data-testid="publication-preview">{JSON.stringify(publication, null, 2)}</pre></details>
            </details>
            {value?.execution_allowed === false && <p role="status">Manifest com automação/publicação: somente inspeção nesta UI. Execute comandos autorizados explicitamente pela CLI.</p>}
            <h3>Execução autorizada</h3><p role="status" data-testid="forge-status">{status || 'not_run'}</p>
            <p>Escopo: manifest fornecido para este projeto. Cancelamento encerra todos os processos ativos. Pausa paralela interrompe a onda; pausa serial aguarda o limite de tarefa/gate. Tarefas interrompidas exigem reconciliação explícita.</p>
            <div className="approval-actions">
                <button disabled={!permitted || status !== 'not_run'} onClick={() => void act('forge-start')}>Iniciar execução</button>
                <button disabled={!permitted || !run.release_id || status === 'running'} onClick={() => void act('forge-resume', reason.trim() ? { reconcile: reason } : {})}>Retomar execução</button>
            </div>
            <h4>Tarefas e concorrência</h4><p>Limite efetivo: {String(object(value?.inspection).effective_max_parallel ?? 1)}</p>
            <ul aria-label="Estados das tarefas">{Object.entries(tasks).map(([id, raw]) => { const task = object(raw); return <li key={id}>{id}: {string(task.status)} · tentativa {String(task.attempts ?? 0)} · PID {String(task.pid ?? '—')}</li>; })}</ul>
            <details><summary>Revisão nativa independente</summary><p>Somente leitura. Achados e uso não aprovam gates nem concedem aceite.</p><button disabled={!permitted} onClick={() => void act('forge-review-plan')}>Prévia da revisão</button><button disabled={!permitted || status === 'running'} onClick={() => void act('forge-review-run')}>Executar revisão nativa</button><pre>{JSON.stringify({ plan: reviewPlan, result: value?.independent_review }, null, 2)}</pre></details>
            <details><summary>Diagnósticos dos gates</summary><pre>{JSON.stringify(state.gate_diagnostics, null, 2)}</pre></details>
            <label htmlFor="reconcile">Motivo de reconciliação (somente se necessário)</label><input id="reconcile" value={reason} onChange={e => setReason(e.target.value)} />
            <details open><summary>Revisão e diff atuais</summary><p>Fingerprint de controle: <code data-testid="control-revision">{revision}</code></p><pre>{string(review.diff) || 'Sem diff de arquivos rastreados.'}</pre><p>Arquivos novos (revise o conteúdo localmente):</p><pre>{string(review.untracked_files) || 'Nenhum'}</pre><pre>{JSON.stringify({ revisions: review.revisions, scopes: review.scopes }, null, 2)}</pre></details>
            <label><input type="checkbox" disabled={!permitted} checked={reviewed === revision} onChange={e => setReviewed(e.target.checked ? revision : '')} />Revisei o diff e o conteúdo desta revisão</label>
            <label htmlFor="reviewer">Identificação de quem revisou</label><input id="reviewer" value={reviewer} onChange={e => setReviewer(e.target.value)} maxLength={200} />
            {Object.entries(tasks).map(([id, raw]) => { const task = object(raw); return task.status === 'awaiting_approval' ? list(task.evidence).filter(e => object(e).kind === 'human' && object(e).outcome !== 'passed').map(rawEvidence => { const evidence = object(rawEvidence); const fields = { task: id, gate: string(evidence.id), reviewer, fingerprint: string(object(task.revision).fingerprint) }; return <div key={`${id}:${string(evidence.id)}`} className="approval"><h4>{id} / {string(evidence.id)}</h4><p>Fingerprint avaliado: <code>{fields.fingerprint}</code></p><button disabled={!permitted || reviewed !== revision || !reviewer.trim() || state.evidence_current !== true} onClick={() => void act('forge-approve', fields)}>Aprovar revisão</button><button disabled={!permitted || !reviewer.trim()} onClick={() => void act('forge-deny', fields)}>Recusar revisão</button></div>; }) : null; })}
            {result && <p role="status">{result}</p>}
            <details><summary>Orçamento, uso e limitações</summary><pre>{JSON.stringify(budget, null, 2)}</pre><p>Saldo e custo monetário desconhecidos. Nenhum teto rígido por inferência é alegado.</p></details>
            <details><summary>Reutilização de validações</summary><pre>{JSON.stringify(cacheEvents, null, 2)}</pre></details>
            <details><summary>Journal de controles e evidências</summary><pre>{JSON.stringify({ operations: value?.control_journal, state }, null, 2)}</pre></details>
        </>}
    </section>;
}
