import { useState } from 'react';
import { type Activity } from './activity';

const operations: Record<string, string> = { add: 'Adicionado', delete: 'Removido', update: 'Editado' };
export function ActivityView({ value, status }: { value: Activity; status: string }) {
    const [open, setOpen] = useState(false);
    return <article className="message activity-card" data-testid="activity-card">
        <details open={open}>
            <summary onClick={e => { e.preventDefault(); setOpen(value => !value); }}><span className="activity-title">{value.title}</span>{status && <span className="activity-status">{status}</span>}<span className="activity-toggle">{open ? 'Ocultar detalhes' : 'Mostrar detalhes'}</span></summary>
            {open && <div className="activity-body">
                {value.truncated && <p className="activity-notice">O cliente enviou mais dados que o limite de exibição. Conteúdo e contagens podem estar incompletos.</p>}
                {value.sections.map(section => <section key={section.label} aria-label={section.label}><h4>{section.label}</h4><pre>{section.text}</pre></section>)}
                {value.files.map((file, i) => <details className="activity-file" key={`${file.path}-${i}`}><summary><span>{file.path}</span><small>{operations[file.operation] || file.operation}{file.added !== undefined ? ` · +${file.added} −${file.removed}` : ''}</small></summary><pre>{file.diff || 'Diff não fornecido pelo cliente.'}</pre></details>)}
                {value.files.some(f => f.added !== undefined) && <p className="activity-notice">Contagens dos hunks recebidos. Alterações não comprovam aceite dos gates.</p>}
                {!value.sections.length && !value.files.length && <p>Sem detalhes informados pelo cliente.</p>}
            </div>}
        </details>
    </article>;
}
