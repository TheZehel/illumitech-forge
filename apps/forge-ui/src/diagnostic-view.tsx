import { list, object, string, type ObjectValue } from './api';

export function DiagnosticView({ value }: { value?: ObjectValue }) {
    const continuity = object(value?.continuity);
    const diagnostic = object(value?.diagnostic ?? continuity.diagnostic);
    if (!string(diagnostic.cause)) return null;
    return <section aria-label="Diagnóstico e continuidade" className="diagnostic-view">
        {string(continuity.objective) && <p><strong>Objetivo:</strong> {string(continuity.objective)}</p>}
        <p><strong>Diagnóstico:</strong> {string(diagnostic.cause)}</p>
        <p><strong>Próximo passo:</strong></p>
        <ul>{list(diagnostic.actions).map((v, i) => <li key={i}>{string(object(v).reason)}</li>)}</ul>
        {list(continuity.coverage_gaps).length > 0 && <p>Cobertura pendente: {list(continuity.coverage_gaps).length} grupos.</p>}
        {Object.keys(continuity).length > 0 && <details><summary>Referências e progresso</summary><pre>{JSON.stringify(continuity, null, 2)}</pre></details>}
    </section>;
}
