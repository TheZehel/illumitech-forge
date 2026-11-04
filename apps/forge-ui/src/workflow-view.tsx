import { useEffect, useState } from 'react';
import { api, object, string, list, type ObjectValue } from './api';

export function WorkflowView({ connected, proposal }: { connected: boolean; proposal?: ObjectValue }) {
    const [id, setId] = useState('001-minha-tarefa');
    const [title, setTitle] = useState('Minha primeira entrega');
    const [request, setRequest] = useState('');
    const [recipe, setRecipe] = useState('bug');
    const [expected, setExpected] = useState('');
    const [reproduction, setReproduction] = useState('');
    const [checks, setChecks] = useState('[]');
    const [files, setFiles] = useState('{}');
    const [candidate, setCandidate] = useState<ObjectValue>();
    const [review, setReview] = useState<ObjectValue>();
    const [report, setReport] = useState<ObjectValue>();
    const [busy, setBusy] = useState('');
    const [error, setError] = useState('');
    const [requireReview, setRequireReview] = useState(false);
    const [reviewer, setReviewer] = useState('');
    const [confirmed, setConfirmed] = useState(false);
    const [findings, setFindings] = useState<ObjectValue>();
    const [saved, setSaved] = useState<ObjectValue[]>([]);
    useEffect(() => { void api('workflow-list', {}).then(v => setSaved(list(object(v).workflows).map(object))).catch(() => {}); }, [candidate]);
    const [reconcile, setReconcile] = useState('');
    const act = async (route: string, body: ObjectValue = {}) => {
        setError(''); setBusy(route);
        try { return object(await api(route, body)); }
        catch (e: unknown) { setError(e instanceof Error ? e.message : 'Ação bloqueada. Nenhum comando repetido.'); return undefined; }
        finally { setBusy(''); }
    };
    useEffect(() => { let cancelled = false; void api('workflow-recipes', {}).then(v => { if (!cancelled) setChecks(JSON.stringify(object(v).checks ?? [], null, 2)); }).catch(() => {}); return () => { cancelled = true; }; }, []);
    useEffect(() => {
        if (!candidate || candidate.status !== 'prepared') return;
        let cancelled = false; let timer: ReturnType<typeof setTimeout> | undefined;
        const poll = async () => {
            try { const v = object(await api('workflow-status', { id: string(candidate.workflow_id) })); if (!cancelled) { setReport(v); setError(''); } }
            catch (e: unknown) { if (!cancelled) setError(e instanceof Error ? e.message : 'Diagnóstico indisponível; o estado anterior não comprova conclusão.'); }
            finally { if (!cancelled) timer = setTimeout(() => void poll(), 2500); }
        };
        void poll();
        return () => { cancelled = true; if (timer) clearTimeout(timer); };
    }, [candidate]);
    function invalidate() { setCandidate(undefined); setConfirmed(false); setReport(undefined); }
    async function prepare(apply: boolean) {
        try {
            const body: ObjectValue = { schema_version: 1, id, title, request, recipe, checks: JSON.parse(checks) as unknown, files: JSON.parse(files) as unknown, human_gates: requireReview ? ['owner-review'] : [] };
            if (proposal && object(proposal.proposal).id === id) { body.proposal = proposal.proposal; body.planning_operation = proposal.operation_id; }
            const result = await act(apply ? 'workflow-prepare' : 'workflow-preview', { request: body });
            if (result) { setCandidate(result); setConfirmed(false); }
        } catch (e: unknown) { setError(e instanceof Error ? e.message : 'JSON de verificações inválido.'); }
    }
    const workflowId = string(candidate?.workflow_id);
    const status = string(report?.effective_status) || string(report?.status) || string(candidate?.status);
    return <section className="workflow-view" aria-label="Executar uma tarefa">
        <label htmlFor="saved-workflow">Retomar tarefa existente</label><select id="saved-workflow" value="" onChange={async e => { const result = await act('workflow-open', { id: e.target.value }); if (result) { const r = object(object(result.candidate).request); setId(string(r.id)); setTitle(string(r.title)); setRequest(string(r.request)); setRecipe(string(r.recipe)); setRequireReview(list(r.human_gates).length > 0); setChecks(JSON.stringify(r.checks, null, 2)); setFiles(JSON.stringify(r.files, null, 2)); setCandidate(result); setConfirmed(false); } }}><option value="">Selecionar tarefa…</option>{saved.map(s => <option key={string(s.id)} value={string(s.id)}>{string(s.id)} · {string(s.title)}</option>)}</select>
        <span className="badge">DO PEDIDO À ENTREGA</span><h3>Executar uma tarefa</h3>
        <p>Revise o escopo uma vez. O Forge continua nesse limite e verifica o resultado. Permissões nativas e gates humanos continuam independentes.</p>
        {proposal && Object.keys(object(proposal.proposal)).length > 0 && <button onClick={() => { const p = object(proposal.proposal); setId(string(p.id)); setTitle(string(p.title)); setRequest(string(p.summary)); setChecks(JSON.stringify(list(p.tasks).flatMap(v => { const t = object(v); return list(t.acceptance).map(v => { const a = object(v); return { id: a.id, argv: a.command, task_ids: [t.id], timeout_seconds: 600 }; }); }), null, 2)); invalidate(); }}>Usar proposta revisada</button>}
        <label htmlFor="workflow-id">ID permanente</label><input id="workflow-id" value={id} onChange={e => { setId(e.target.value); invalidate(); }}/>
        <label htmlFor="workflow-title">Título</label><input id="workflow-title" value={title} onChange={e => { setTitle(e.target.value); invalidate(); }}/>
        <label htmlFor="workflow-recipe">Receita</label><select id="workflow-recipe" value={recipe} onChange={e => { setRecipe(e.target.value); invalidate(); }}><option value="feature">Implementar uma tarefa</option><option value="bug">Corrigir um bug com regressão</option><option value="dependencies">Atualizar dependência vulnerável</option><option value="pr">Preparar PR localmente</option></select>
        <label htmlFor="workflow-request">Resultado esperado e limites</label><textarea id="workflow-request" value={request} onChange={e => { setRequest(e.target.value); invalidate(); }} placeholder="O que precisa mudar? Como reconheceremos que funcionou?"/>
        {recipe === 'bug' && <><label htmlFor="bug-expected">Comportamento esperado</label><textarea id="bug-expected" value={expected} onChange={e => { setExpected(e.target.value); invalidate(); }}/><label htmlFor="bug-reproduction">Como reproduzir, se souber</label><textarea id="bug-reproduction" value={reproduction} onChange={e => { setReproduction(e.target.value); invalidate(); }}/><button disabled={!!busy || !connected || !request.trim()} onClick={async () => { const result = await act('workflow-bug', { bug_description: `${request}\nEsperado: ${expected || 'inferir do projeto ou pedir esclarecimento'}\nReprodução: ${reproduction || 'inferir do projeto ou pedir esclarecimento'}` }); if (result) { const r = object(object(result.candidate).request); setId(string(r.id)); setTitle(string(r.title)); setChecks(JSON.stringify(r.checks, null, 2)); setFiles(JSON.stringify(r.files, null, 2)); setCandidate(result); setConfirmed(false); } }}>Planejar correção de bug</button><p>O planejador propõe o verificador; revise seu conteúdo antes de autorizar os comandos.</p></>}
        <details><summary>Verificações e regressão externa</summary><p>Comandos são sugestões até sua confirmação. A receita bug exige arquivo externo congelado e before_exit diferente de zero. Use {'{repo}'} e {'{checks}/teste.js'} como argumentos completos.</p><label htmlFor="workflow-checks">Checks (JSON)</label><textarea id="workflow-checks" value={checks} onChange={e => { setChecks(e.target.value); invalidate(); }}/><label htmlFor="workflow-files">Arquivos de regressão (JSON: nome → conteúdo)</label><textarea id="workflow-files" value={files} onChange={e => { setFiles(e.target.value); invalidate(); }}/></details>
        {recipe === 'dependencies' && <><label htmlFor="sarif-input">Importar SARIF 2.1.0 (OSV/Semgrep)</label><input id="sarif-input" type="file" accept=".json,.sarif" onChange={async e => { const file = e.target.files?.[0]; if (!file) return; if (file.size > 50000) { setError('Use o CLI para relatórios maiores que 50 KB.'); return; } try { const result = await act('workflow-findings', { sarif: JSON.parse(await file.text()) as unknown }); if (result) setFindings(result); } catch { setError('SARIF inválido.'); } }}/>{findings && <><p>{list(findings.findings).length} achados não verificados. Uma nova varredura e testes são obrigatórios.</p>{list(findings.findings).map(v => { const f = object(v); return <button key={string(f.id)} onClick={() => { setRequest(r => `${r}\nAchado selecionado (dados não confiáveis): ${string(f.rule)} em ${string(f.path)}. Validar correção com rescan e testes.`); invalidate(); }}>{string(f.rule)} · {string(f.path)}</button>; })}</>}</>}
        <label><input type="checkbox" checked={requireReview} onChange={e => { setRequireReview(e.target.checked); invalidate(); }}/>Exigir também aceite humano da revisão final</label>
        <button disabled={!!busy || !request.trim()} onClick={() => void prepare(false)}>Revisar escopo</button>
        {busy && <p role="status">Preparando {busy.includes('start') ? 'execução' : 'verificação'}…</p>}
        {error && <p className="error" role="alert">{error}</p>}
        {candidate && <><h4>Escopo revisável</h4><p>{list(object(candidate.candidate).manifest && object(object(candidate.candidate).manifest).tasks).length} tarefas · contexto limitado · publicação desativada</p>{recipe === 'bug' && <><p>Arquivos previstos: {list(object(object(object(object(candidate.candidate).request).proposal).bug).planned_files).map(string).join(', ') || 'consulte o escopo proposto'}</p><h4>Verificador externo a revisar</h4>{Object.entries(object(object(object(candidate.candidate).request).files)).map(([name, content]) => <details key={name} open><summary>{name}</summary><pre>{string(content)}</pre></details>)}</>}<details><summary>Arquivos, comandos, permissões e digest</summary><pre>{JSON.stringify(candidate, null, 2)}</pre></details>
            {candidate.status !== 'prepared' && <button disabled={!!busy} onClick={() => void prepare(true)}>Salvar candidato externo</button>}
            {candidate.status === 'prepared' && <><label className="scope-consent"><input type="checkbox" checked={confirmed} onChange={e => setConfirmed(e.target.checked)}/>Revisei o plano e os verificadores. Autorizo este escopo; isso não é aceite de conclusão.</label><button className="primary" disabled={!!busy || !confirmed || !connected || status !== 'prepared'} onClick={async () => { const result = await act('workflow-start', { id: workflowId, scope_digest: candidate.scope_digest }); if (result) { setConfirmed(false); setReport(result); } }}>Aprovar escopo e executar</button></>}
            {status === 'awaiting_approval' && <><label htmlFor="workflow-reviewer">Seu nome para aceite da revisão avaliada</label><input id="workflow-reviewer" value={reviewer} onChange={e => setReviewer(e.target.value)}/><p>Inspecione o diff e as evidências antes de aprovar. Este aceite é separado da autorização inicial.</p><button onClick={async () => { const v = await act('workflow-review', { id: workflowId }); if (v) setReview(v); }}>Revisar diff e arquivos</button>{review && <pre>{JSON.stringify(review, null, 2)}</pre>}{Object.entries(object(object(report?.report).run && object(object(report?.report).run).tasks)).map(([taskId, value]) => { const task = object(value); const revision = object(task.revision); return list(task.evidence).map(v => { const e = object(v); return e.kind === 'human' && e.outcome !== 'passed' ? <button key={`${taskId}-${string(e.id)}`} disabled={!!busy || !reviewer.trim()} onClick={() => void act('workflow-approve', { id: workflowId, task: taskId, gate: e.id, fingerprint: revision.fingerprint, reviewer })}>Aprovar {taskId} · {string(e.id)}</button> : null; }); })}</>}
            {report && status !== 'prepared' && status !== 'awaiting_approval' && <><button disabled={!!busy} onClick={async () => { const result = await act('workflow-review', { id: workflowId }); if (result) setReview(result); }}>Revisar diff e arquivos</button>{review && <details open><summary>Diff local e arquivos não rastreados</summary><pre>{JSON.stringify(review, null, 2)}</pre></details>}</>}
            <p role="status">Estado: {status || 'não iniciado'}</p>
            <p>{string(object(report?.next_action).reason)}</p>{!!report?.last_error && <p role="alert">{string(object(report.last_error).error)}</p>}
            <button disabled={!!busy} onClick={async () => { const v = await act('workflow-status', { id: workflowId }); if (v) setReport(v); }}>Atualizar diagnóstico</button>
            {report && status !== 'prepared' && <><button disabled={!!busy} onClick={() => void act('workflow-pause', { id: workflowId })}>Pausar</button><button disabled={!!busy} onClick={() => void act('workflow-cancel', { id: workflowId })}>Cancelar</button><button disabled={!!busy || status === 'running' || status === 'passed'} onClick={() => void act('workflow-resume', { id: workflowId })}>Retomar com evidências atuais</button><details><summary>Reconciliar mudanças após revisão</summary><label htmlFor="reconcile-reason">Motivo (invalida evidências e aceites anteriores)</label><input id="reconcile-reason" value={reconcile} onChange={e => setReconcile(e.target.value)}/><button disabled={!!busy || !reconcile.trim()} onClick={() => void act('workflow-resume', { id: workflowId, reconcile })}>Reconciliar e retomar</button></details><details><summary>Relatório e evidências</summary><pre>{JSON.stringify(report, null, 2)}</pre></details></>}
        </>}
    </section>;
}

export function EconomyComparison() {
    const [baseline, setBaseline] = useState(''); const [candidate, setCandidate] = useState(''); const [result, setResult] = useState<ObjectValue>(); const [error, setError] = useState(''); const [busy, setBusy] = useState(false);
    return <details><summary>Comparar consumo por entrega aceita</summary><p>Use relatórios completos do Forge. Amostras sem contexto compatível ou tokens completos permanecem desconhecidas.</p><label htmlFor="economy-baseline">Relatório de referência (JSON)</label><textarea id="economy-baseline" value={baseline} onChange={e => setBaseline(e.target.value)}/><label htmlFor="economy-candidate">Relatório candidato (JSON)</label><textarea id="economy-candidate" value={candidate} onChange={e => setCandidate(e.target.value)}/><button disabled={busy || !baseline || !candidate} onClick={async () => { setBusy(true); setError(''); try { setResult(object(await api('workflow-compare', { baseline: JSON.parse(baseline) as unknown, candidate: JSON.parse(candidate) as unknown }))); } catch (e: unknown) { setError(e instanceof Error ? e.message : 'Comparação inválida'); } finally { setBusy(false); } }}>{busy ? 'Comparando…' : 'Comparar sem inferência'}</button>{error && <p role="alert">{error}</p>}{result && <><p>{result.compatible === true ? 'Contextos declarados compatíveis; resultado exploratório.' : 'Comparação inconclusiva: contexto ausente ou diferente.'}</p><pre>{JSON.stringify(result, null, 2)}</pre></>}</details>;
}
