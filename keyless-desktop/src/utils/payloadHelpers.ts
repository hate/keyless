/**
 * Utility functions for extracting common fields from event payloads.
 * Reduces repetitive type guard code across event handlers.
 */

/**
 * Safely extracts modelId from an event payload.
 * Returns null if payload is invalid or modelId is missing/invalid.
 */
export function extractModelId(payload: unknown): string | null {
    // Type guard: ensure payload is an object.
    if (!payload || typeof payload !== 'object') return null;
    // Extract modelId field if it exists and is a string.
    const modelId = 'modelId' in payload && typeof payload.modelId === 'string' ? payload.modelId : null;
    return modelId;
}

/**
 * Safely extracts a string field from an event payload.
 * Returns null if payload is invalid or field is missing/invalid.
 */
export function extractStringField(payload: unknown, field: string): string | null {
    // Type guard: ensure payload is an object.
    if (!payload || typeof payload !== 'object') return null;
    // Extract field if it exists and is a string.
    const value = field in payload && typeof (payload as Record<string, unknown>)[field] === 'string'
        ? (payload as Record<string, unknown>)[field] as string
        : null;
    return value;
}

/**
 * Safely extracts an object field from an event payload.
 * Returns null if payload is invalid or field is missing/invalid.
 */
export function extractObjectField(payload: unknown, field: string): Record<string, unknown> | null {
    // Type guard: ensure payload is an object.
    if (!payload || typeof payload !== 'object') return null;
    // Extract field value.
    const value = (payload as Record<string, unknown>)[field];
    // Type guard: ensure value is a non-null object (excludes arrays, null, primitives).
    return value && typeof value === 'object' && value !== null ? value as Record<string, unknown> : null;
}

