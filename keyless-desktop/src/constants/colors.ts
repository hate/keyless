/**
 * Color constants for use in JavaScript/TypeScript code.
 * 
 * IMPORTANT: Only use this file for colors that CANNOT be Tailwind classes:
 * - Inline styles: `style={{ borderColor: COLORS.sink.paste }}`
 * - Canvas API: `ctx.fillStyle = COLORS.canvas.gradientTop`
 * - Dynamic color selection that requires JavaScript values
 * 
 * For all other cases, use Tailwind classes from tailwind.config.ts:
 * - `bg-bgCard`, `text-textPrimary`, `border-border`, etc.
 */
export const COLORS = {
	// Status colors (for inline styles and dynamic selection)
	status: {
		success: '#50fa7b',
		error: '#f87171',
		warning: '#f1fa8c',
		info: '#4ea3ff',
	},
	// Output sink colors (for ColoredButtonGroup inline styles)
	sink: {
		paste: '#50fa7b',
		clipboard: '#f1fa8c',
		file: '#4ea3ff',
	},
	// Error colors (for inline styles)
	error: {
		default: '#ff5555',
		light: '#ff6666',
	},
	// Canvas API colors (for AudioVisualizer gradients)
	canvas: {
		gradientTop: '#ff5555',    // Red
		gradientMiddle: '#f1fa8c', // Yellow
		gradientBottom: '#50fa7b', // Green
		inactive: '#2a2a2a',       // Grey for inactive bars
	},
} as const;

