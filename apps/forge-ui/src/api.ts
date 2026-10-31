export type ObjectValue = Record<string, unknown>;
export const object = (v: unknown): ObjectValue => typeof v === 'object' && v !== null && !Array.isArray(v) ? v as ObjectValue : {};
export const string = (v: unknown): string => typeof v === 'string' ? v : '';
export const list = (v: unknown): unknown[] => Array.isArray(v) ? v : [];
export interface Event {
    method: string;
    params: ObjectValue;
    id?: unknown;
    choices: unknown[];
}
export interface Session {
    id: string;
    title: string;
    events: Event[];
    active_turn: string | null;
    usage: ObjectValue;
    effective: ObjectValue;
    loaded: boolean;
}
export interface Snapshot {
    connected: boolean;
    error: string;
    project: string;
    authenticated: boolean;
    sessions: Session[];
    approvals: Event[];
    forge: ObjectValue;
}
const event = (v: unknown): Event => { const o = object(v); return { method: string(o.method), params: object(o.params), id: o.id, choices: list(o.choices) }; };
export function snapshot(v: unknown): Snapshot {
    const o = object(v);
    if (o.schema_version !== 1 || typeof o.connected !== 'boolean' || !Array.isArray(o.sessions))
        throw new Error('Resposta local incompatível. Reconecte o cliente.');
    return { connected: o.connected, error: string(o.error), project: string(object(o.project).name), authenticated: object(o.account).authenticated === true, sessions: o.sessions.map(v => { const s = object(v); if (!string(s.id))
            throw new Error('Sessão sem identidade'); return { id: string(s.id), title: string(s.title), events: list(s.events).map(event), active_turn: typeof s.active_turn === 'string' ? s.active_turn : null, usage: object(s.usage), effective: object(s.effective), loaded: s.loaded === true }; }), approvals: list(o.approvals).map(event), forge: object(o.forge) };
}
let capability = '';
export async function bootstrap(): Promise<void> {
    const fragment = new URLSearchParams(location.hash.slice(1));
    const incoming = fragment.get('session');
    if (incoming) {
        sessionStorage.setItem('forge.capability', incoming);
        history.replaceState(null, '', location.pathname);
    }
    capability = sessionStorage.getItem('forge.capability') ?? '';
    if (!/^[a-f0-9]{64}$/.test(capability))
        throw new Error('Acesso bloqueado. Abra o endereço local emitido pelo Forge no terminal.');
    const r = await fetch('/api/connect', { method: 'POST', headers: { 'X-Forge-Token': capability } });
    if (!r.ok)
        throw new Error('Sessão local expirada. Abra o endereço atual emitido pelo Forge.');
}
export async function api(path: string, body?: ObjectValue, signal?: AbortSignal): Promise<unknown> {
    const reading = body === undefined;
    const controller = new AbortController();
    let timedOut = false;
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort();
    else signal?.addEventListener('abort', abort, { once: true });
    const timer = reading ? setTimeout(() => { timedOut = true; controller.abort(); }, 5000) : undefined;
    try {
        const r = await fetch(`/api/${path}`, { method: reading ? 'GET' : 'POST', headers: { 'X-Forge-CSRF': capability, ...(!reading ? { 'Content-Type': 'application/json' } : {}) }, body: reading ? undefined : JSON.stringify({ ...body, operation_id: crypto.randomUUID() }), signal: controller.signal });
        const v: unknown = await r.json();
        if (!r.ok)
            throw new Error(string(object(v).error) || 'Ponte local indisponível. Nenhuma ação foi repetida.');
        return v;
    } catch (error: unknown) {
        if (timedOut) throw new Error('A consulta local excedeu 5 segundos. Os controles de emergência continuam disponíveis.');
        throw error;
    } finally {
        clearTimeout(timer);
        signal?.removeEventListener('abort', abort);
    }
}
