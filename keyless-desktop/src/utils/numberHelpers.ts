/**
 * Utility functions for number handling and validation.
 */

/**
 * Converts a value to a number, returning 0 if invalid or NaN.
 * Useful for safely parsing numeric values from API responses.
 */
export function numberOrZero(value: unknown): number {
    // Type guard: ensure value is a valid number (not NaN, not undefined/null).
    // Returns 0 for invalid values (safe default for numeric operations).
    return typeof value === 'number' && !Number.isNaN(value) ? value : 0;
}

