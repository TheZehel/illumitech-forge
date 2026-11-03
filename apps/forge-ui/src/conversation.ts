import { type Event, list, object, string } from './api';
import { activity, bounded, type Activity } from './activity';
export interface Message {
    id: string;
    kind: string;
    text: string;
    status: string;
    activity?: Activity;
}
export function messages(events: Event[]): Message[] {
    const result = new Map<string, Message>();
    let echo: Message | undefined;
    const aliases = new Map<string, string>();
    const items = new Map<string, Record<string, unknown>>();
    const putActivity = (id: string, item: Record<string, unknown>) => {
        items.set(id, item);
        const failed = item.error != null || object(item.result).isError === true || item.success === false || typeof item.exitCode === 'number' && item.exitCode !== 0;
        result.set(id, { id, kind: string(item.type), text: '', status: failed ? 'failed' : string(item.status), activity: activity(item) });
    };
    events.forEach((e, index) => {
        const p = e.params;
        const item = object(p.item);
        const id = string(item.id) || string(p.itemId) || `event-${index}`;
        if (e.method === 'bridge/user') {
            echo = { id, kind: 'userMessage', text: string(p.text), status: '' };
            result.set(id, echo);
        }
        else if (e.method === 'item/agentMessage/delta') {
            const old = result.get(id);
            result.set(id, { id, kind: 'agentMessage', text: (old?.text ?? '') + string(p.delta), status: 'streaming' });
        }
        else if (e.method === 'item/started' || e.method === 'item/completed') {
            const kind = string(item.type);
            const text = string(item.text) || string(item.aggregatedOutput) || string(item.command) || list(item.content).map(v => string(object(v).text)).join('\n') || (kind === 'fileChange' ? JSON.stringify(item.changes, null, 2) : '');
            if (kind === 'userMessage' && aliases.has(id)) return;
            if (kind === 'userMessage' && echo?.text === text) {
                aliases.set(id, echo.id);
                result.delete(id);
                echo = undefined;
                return;
            }
            if (kind && !['agentMessage', 'userMessage', 'reasoning'].includes(kind)) {
                putActivity(id, { ...items.get(id), ...item, status: string(item.status) || (e.method === 'item/started' ? 'inProgress' : '') });
            } else if (kind && (kind !== 'userMessage' || text))
                result.set(id, { id, kind, text, status: string(item.status) });
        }
        else if (e.method === 'item/commandExecution/outputDelta' || e.method === 'item/fileChange/outputDelta') {
            const old = items.get(id) ?? { type: e.method.includes('commandExecution') ? 'commandExecution' : 'fileChange', status: 'inProgress' };
            putActivity(id, { ...old, aggregatedOutput: bounded(string(old.aggregatedOutput) + string(p.delta)) });
        }
        else if (e.method === 'item/mcpToolCall/progress') {
            const old = items.get(id) ?? { type: 'mcpToolCall', status: 'inProgress' };
            putActivity(id, { ...old, progress: bounded([string(old.progress), string(p.message)].filter(Boolean).join('\n')) });
        }
        else if (e.method === 'turn/diff/updated')
            putActivity(`diff-${string(p.turnId) || id}`, { type: 'diff', diff: string(p.diff), status: 'proposed' });
        else if (e.method === 'bridge/decision')
            result.set(id, { id, kind: 'decision', text: typeof p.decision === 'string' ? p.decision : JSON.stringify(p.decision), status: 'submitted' });
        else if (e.method === 'error' || e.method === 'bridge/truncated')
            result.set(id, { id, kind: 'error', text: string(object(p.error).message) || string(p.reason), status: 'blocked' });
        else if (e.method === 'turn/plan/updated')
            result.set(id, { id, kind: 'plan', text: JSON.stringify(p.plan, null, 2), status: 'proposed' });
        else if (e.method === 'turn/completed') {
            echo = undefined;
            result.set(id, { id, kind: 'turn', text: `Turno ${string(object(p.turn).status)}${object(p.turn).error ? `: ${JSON.stringify(object(p.turn).error)}` : ''}`, status: string(object(p.turn).status) });
        }
    });
    return [...result.values()];
}
export function choiceLabel(choice: unknown): string {
    switch (choice) {
        case 'accept': return 'Permitir uma vez';
        case 'acceptForSession': return 'Permitir nesta sessão';
        case 'decline': return 'Recusar';
        case 'cancel': return 'Recusar e interromper';
        default: return typeof choice === 'string' ? choice : JSON.stringify(choice);
    }
}
