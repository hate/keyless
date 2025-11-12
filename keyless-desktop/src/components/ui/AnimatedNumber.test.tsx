import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { AnimatedNumber } from './AnimatedNumber';

// Mock requestAnimationFrame
beforeEach(() => {
	vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
		return setTimeout(() => cb(performance.now()), 16);
	});
	vi.stubGlobal('cancelAnimationFrame', (id: number) => {
		clearTimeout(id);
	});
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('AnimatedNumber', () => {
	it('renders initial value', () => {
		render(<AnimatedNumber value={100} />);
		const element = screen.getByText('100');
		expect(element).toBeTruthy();
	});

	it('formats numbers with locale string', () => {
		render(<AnimatedNumber value={1000} />);
		const element = screen.getByText('1,000');
		expect(element).toBeTruthy();
	});

	it('applies custom className', () => {
		const { container } = render(<AnimatedNumber value={42} className="custom-class" />);
		const element = container.querySelector('.custom-class');
		expect(element).toBeTruthy();
	});

	it('has data-value attribute', () => {
		const { container } = render(<AnimatedNumber value={100} />);
		const element = container.querySelector('[data-value]');
		expect(element).toBeTruthy();
		expect(element?.getAttribute('data-value')).toBe('0'); // initial value
	});
});

