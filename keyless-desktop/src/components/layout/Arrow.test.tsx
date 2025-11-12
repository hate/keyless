import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Arrow } from './Arrow';

describe('Arrow', () => {
	it('renders arrow pointing up by default', () => {
		const { container } = render(<Arrow />);
		// Check that the arrow container exists
		expect(container.firstChild).toBeTruthy();
	});

	it('renders arrow pointing up when direction="up"', () => {
		const { container } = render(<Arrow direction="up" />);
		// The arrow should have border-b classes (pointing up)
		const arrow = container.querySelector('.border-b-\\[14px\\]');
		expect(arrow).toBeTruthy();
	});

	it('renders arrow pointing down when direction="down"', () => {
		const { container } = render(<Arrow direction="down" />);
		// The arrow should have border-t classes (pointing down)
		const arrow = container.querySelector('.border-t-\\[14px\\]');
		expect(arrow).toBeTruthy();
	});

	it('has correct structure with outer and inner arrows', () => {
		const { container } = render(<Arrow direction="up" />);
		// Should have both outer (border) and inner (fill) arrows
		const arrows = container.querySelectorAll('[class*="border"]');
		expect(arrows.length).toBeGreaterThan(0);
	});
});

