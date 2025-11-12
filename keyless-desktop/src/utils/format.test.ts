import { describe, it, expect } from 'vitest';
import { formatBytes, formatEta, formatDuration } from './format';

describe('formatBytes', () => {
	it('formats bytes as MB for values < 1000 MB', () => {
		expect(formatBytes(1_000_000)).toBe('1.0 MB');
		expect(formatBytes(500_000_000)).toBe('500.0 MB');
		expect(formatBytes(999_999_999)).toBe('1000.0 MB'); // 999.999999 MB rounds to 1000.0 MB
	});

	it('formats bytes as GB for values >= 1000 MB', () => {
		expect(formatBytes(1_000_000_000)).toBe('1.00 GB');
		expect(formatBytes(2_500_000_000)).toBe('2.50 GB');
		expect(formatBytes(5_000_000_000)).toBe('5.00 GB');
	});

	it('handles edge cases', () => {
		expect(formatBytes(0)).toBe('0 MB');
		expect(formatBytes(undefined)).toBe('?');
		expect(formatBytes(NaN)).toBe('?');
		expect(formatBytes(-100)).toBe('0 MB');
	});

	it('handles small values correctly', () => {
		expect(formatBytes(1)).toBe('0.0 MB');
		expect(formatBytes(1000)).toBe('0.0 MB');
		expect(formatBytes(999_999)).toBe('1.0 MB'); // 0.999999 MB rounds to 1.0 MB
	});
});

describe('formatEta', () => {
	it('formats seconds < 60 as "Xs"', () => {
		expect(formatEta(0)).toBe('0s');
		expect(formatEta(30)).toBe('30s');
		expect(formatEta(59)).toBe('59s');
	});

	it('formats seconds >= 60 as "Xm Ys"', () => {
		expect(formatEta(60)).toBe('1m 0s');
		expect(formatEta(90)).toBe('1m 30s');
		expect(formatEta(125)).toBe('2m 5s');
		expect(formatEta(3661)).toBe('61m 1s');
	});

	it('handles edge cases', () => {
		expect(formatEta(undefined)).toBe('');
		expect(formatEta(NaN)).toBe('');
		expect(formatEta(-10)).toBe('0s');
		expect(formatEta(30.7)).toBe('31s'); // rounds up
		expect(formatEta(30.3)).toBe('30s'); // rounds down
	});
});

describe('formatDuration', () => {
	it('formats durations < 60s as "Xs"', () => {
		expect(formatDuration(1000)).toBe('1s');
		expect(formatDuration(30000)).toBe('30s');
		expect(formatDuration(59000)).toBe('59s');
	});

	it('formats durations >= 60s as "Xm Ys"', () => {
		expect(formatDuration(60000)).toBe('1m 0s');
		expect(formatDuration(90000)).toBe('1m 30s');
		expect(formatDuration(125000)).toBe('2m 5s');
	});

	it('formats durations >= 3600s as "Xh Ym"', () => {
		expect(formatDuration(3600000)).toBe('1h 0m');
		expect(formatDuration(5400000)).toBe('1h 30m');
		expect(formatDuration(7200000)).toBe('2h 0m');
	});

	it('handles edge cases', () => {
		expect(formatDuration(0)).toBe('0s');
		expect(formatDuration(undefined)).toBe('0s');
		expect(formatDuration(-1000)).toBe('0s');
	});
});

