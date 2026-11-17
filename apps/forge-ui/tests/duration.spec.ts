import { test, expect } from '@playwright/test';
import { durationRange } from '../src/duration';

test('readable duration bounds, including transitions and unavailable data', () => {
    expect(durationRange(9, 400)).toBe('9 segundos a 6 minutos e 40 segundos');
    expect(durationRange(60, 120)).toBe('1 minuto a 2 minutos');
    expect(durationRange(3599.2, 3600.1)).toBe('59 minutos e 59 segundos a 1 hora e 1 segundo');
    expect(durationRange(0, 0.1)).toBe('0 segundos a 1 segundo');
    expect(durationRange(3600, 90061)).toBe('1 hora a 1 dia, 1 hora, 1 minuto e 1 segundo');
    for (const [lower, upper] of [[null, 60], [9, undefined], [9, NaN], [-1, 2], [60, 1], [1, Infinity]]) {
        expect(durationRange(lower, upper)).toBeUndefined();
    }
});

test('communicates days and natural singular/plural without artificial ranges', () => {
    expect(durationRange(630, 630)).toBe('10 minutos e 30 segundos');
    expect(durationRange(86400, 86400)).toBe('1 dia');
    expect(durationRange(90000, 90000)).toBe('1 dia e 1 hora');
    expect(durationRange(124200, 124200)).toBe('1 dia, 10 horas e 30 minutos');
    expect(durationRange(172801, 172801)).toBe('2 dias e 1 segundo');
    expect(durationRange(0, 0)).toBe('0 segundos');
});
