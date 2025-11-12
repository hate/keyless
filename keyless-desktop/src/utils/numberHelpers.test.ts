import { describe, it, expect } from 'vitest';
import { numberOrZero } from './numberHelpers';

describe('numberOrZero', () => {
	it('returns the number for valid numbers', () => {
		expect(numberOrZero(0)).toBe(0);
		expect(numberOrZero(42)).toBe(42);
		expect(numberOrZero(-100)).toBe(-100);
		expect(numberOrZero(3.14)).toBe(3.14);
	});

	it('returns 0 for NaN', () => {
		expect(numberOrZero(NaN)).toBe(0);
	});

	it('returns 0 for undefined', () => {
		expect(numberOrZero(undefined)).toBe(0);
	});

	it('returns 0 for null', () => {
		expect(numberOrZero(null)).toBe(0);
	});

	it('returns 0 for non-number types', () => {
		expect(numberOrZero('42')).toBe(0);
		expect(numberOrZero('hello')).toBe(0);
		expect(numberOrZero({})).toBe(0);
		expect(numberOrZero([])).toBe(0);
		expect(numberOrZero(true)).toBe(0);
		expect(numberOrZero(false)).toBe(0);
	});
});

