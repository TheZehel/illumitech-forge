/** Natural Portuguese duration; round bounds outwards and retain JSON seconds. */
export function durationRange(lower: unknown, upper: unknown): string | undefined {
    if (typeof lower !== 'number' || typeof upper !== 'number'
        || !Number.isFinite(lower) || !Number.isFinite(upper)
        || lower < 0 || upper < lower) return undefined;
    const format = (total: number): string => {
        const units: [number, string, string][] = [
            [Math.floor(total / 86400), 'dia', 'dias'],
            [Math.floor(total % 86400 / 3600), 'hora', 'horas'],
            [Math.floor(total % 3600 / 60), 'minuto', 'minutos'],
            [total % 60, 'segundo', 'segundos'],
        ];
        const parts = units.filter(([value]) => value > 0)
            .map(([value, singular, plural]) => `${value} ${value === 1 ? singular : plural}`);
        const last = parts.pop();
        if (last === undefined) return '0 segundos';
        return parts.length ? `${parts.join(', ')} e ${last}` : last;
    };
    const lo = format(Math.floor(lower));
    const hi = format(Math.ceil(upper));
    return lo === hi ? lo : `${lo} a ${hi}`;
}
