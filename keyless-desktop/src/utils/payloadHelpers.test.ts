import { describe, it, expect } from 'vitest';
import { extractModelId, extractStringField, extractObjectField } from './payloadHelpers';

describe('extractModelId', () => {
	it('extracts modelId from valid payload', () => {
		expect(extractModelId({ modelId: 'test-model' })).toBe('test-model');
		expect(extractModelId({ modelId: 'another-model', other: 'data' })).toBe('another-model');
	});

	it('returns null for invalid payloads', () => {
		expect(extractModelId(null)).toBeNull();
		expect(extractModelId(undefined)).toBeNull();
		expect(extractModelId({})).toBeNull();
		expect(extractModelId({ modelId: 123 })).toBeNull();
		expect(extractModelId({ modelId: null })).toBeNull();
		expect(extractModelId('not an object')).toBeNull();
	});
});

describe('extractStringField', () => {
	it('extracts string field from valid payload', () => {
		expect(extractStringField({ name: 'test' }, 'name')).toBe('test');
		expect(extractStringField({ error: 'error message' }, 'error')).toBe('error message');
	});

	it('returns null for missing or invalid fields', () => {
		expect(extractStringField({}, 'missing')).toBeNull();
		expect(extractStringField({ name: 123 }, 'name')).toBeNull();
		expect(extractStringField(null, 'name')).toBeNull();
		expect(extractStringField('not an object', 'name')).toBeNull();
	});
});

describe('extractObjectField', () => {
	it('extracts object field from valid payload', () => {
		const payload = { data: { key: 'value' } };
		expect(extractObjectField(payload, 'data')).toEqual({ key: 'value' });
	});

	it('returns null for non-object values', () => {
		expect(extractObjectField({ data: 'string' }, 'data')).toBeNull();
		expect(extractObjectField({ data: 123 }, 'data')).toBeNull();
		expect(extractObjectField({ data: null }, 'data')).toBeNull();
		// Note: Arrays are objects in JavaScript, so [] is returned as an object
		// The implementation doesn't exclude arrays, it just checks for object type
		expect(extractObjectField({ data: [] }, 'data')).not.toBeNull();
		expect(extractObjectField({}, 'missing')).toBeNull();
		expect(extractObjectField(null, 'data')).toBeNull();
	});
});

