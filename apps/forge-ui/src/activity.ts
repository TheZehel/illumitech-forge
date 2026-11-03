import { list, object, string, type ObjectValue } from './api';

export interface ActivitySection { label: string; text: string }
export interface ActivityFile { path: string; operation: string; diff: string; added?: number; removed?: number }
export interface Activity {
    title: string;
    sections: ActivitySection[];
    files: ActivityFile[];
    truncated: boolean;
}
const DISPLAY_LIMIT = 24000;
export function bounded(text: string): string {
    return text.length > DISPLAY_LIMIT ? `${text.slice(0, DISPLAY_LIMIT)}\n… Exibição limitada a ${DISPLAY_LIMIT} caracteres.` : text;
}
const pretty = (value: unknown): string => value === undefined || value === null ? '' : bounded(typeof value === 'string' ? value : JSON.stringify(value, null, 2));
// Count only received unified-diff hunks, never headers or invented file contents.
export function diffCounts(diff: string): { added?: number; removed?: number } {
    let hunk = false, observed = false, added = 0, removed = 0;
    for (const line of diff.split('\n')) {
        if (line.startsWith('diff ') || line.startsWith('*** ')) hunk = false;
        if (line.startsWith('@@ ')) { hunk = true; observed = true; }
        else if (hunk && line.startsWith('+')) added++;
        else if (hunk && line.startsWith('-')) removed++;
    }
    return observed ? { added, removed } : {};
}
export function activity(item: ObjectValue): Activity {
    const kind = string(item.type);
    const sections: ActivitySection[] = [];
    const add = (label: string, value: unknown) => { const text = pretty(value); if (text) sections.push({ label, text }); };
    let title = 'Atividade do cliente';
    let files: ActivityFile[] = [];
    if (kind === 'mcpToolCall' || kind === 'dynamicToolCall') {
        title = `Chamada · ${[string(item.server), string(item.tool)].filter(Boolean).join('.') || 'Ferramenta'}`;
        add('Argumentos', item.arguments);
        add('Resultado', item.result);
        add('Erro', item.error);
        add('Progresso', item.progress);
        if (kind === 'dynamicToolCall') add('Conteúdo retornado', item.contentItems);
    } else if (kind === 'commandExecution') {
        title = `Comando · ${string(item.command).split('\n')[0].slice(0, 160) || 'Execução nativa'}`;
        add('Comando', item.command);
        add('Diretório', item.cwd);
        if (typeof item.exitCode === 'number') add('Código de saída', item.exitCode);
        add('Saída', item.aggregatedOutput);
    } else if (kind === 'fileChange') {
        files = list(item.changes).filter(change => typeof change === 'object' && change !== null && !Array.isArray(change)).map(change => {
            const c = object(change), diff = string(c.diff);
            return { path: string(c.path) || 'Arquivo sem caminho informado', operation: string(object(c.kind).type) || string(c.kind) || 'update', diff: bounded(diff), ...diffCounts(diff) };
        });
        const counted = files.length > 0 && files.every(f => f.added !== undefined);
        const total = typeof item.forgeOriginalFileCount === 'number' && item.forgeOriginalFileCount > files.length ? `${files.length} de ${item.forgeOriginalFileCount}` : String(files.length);
        title = `Alterações · ${total} ${files.length === 1 && total === '1' ? 'arquivo' : 'arquivos'}${counted ? ` (+${files.reduce((n, f) => n + (f.added ?? 0), 0)} −${files.reduce((n, f) => n + (f.removed ?? 0), 0)})` : ''}`;
        add('Saída', item.aggregatedOutput);
    } else if (kind === 'diff') {
        title = 'Diff proposto pelo cliente'; add('Diff', item.diff);
    } else if (kind === 'webSearch') {
        title = 'Pesquisa'; add('Consulta', item.query); add('Ação', item.action);
    } else {
        title = kind === 'reasoning' ? 'Raciocínio disponibilizado pelo cliente' : 'Atividade do cliente';
        add('Dados informados pelo cliente', item);
    }
    const truncated = item.forgeDisplayTruncated === true;
    return { title: title + (truncated ? ' · parcial' : ''), sections, files, truncated };
}
