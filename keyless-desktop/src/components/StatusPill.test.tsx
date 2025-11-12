import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusPill } from './StatusPill';

describe('StatusPill', () => {
	it('renders active state with default labels', () => {
		render(<StatusPill active={true} />);
		expect(screen.getByText('Active')).toBeTruthy();
	});

	it('renders inactive state with default labels', () => {
		render(<StatusPill active={false} />);
		expect(screen.getByText('Idle')).toBeTruthy();
	});

	it('uses custom labels', () => {
		render(<StatusPill active={true} activeLabel="Listening" inactiveLabel="Stopped" />);
		expect(screen.getByText('Listening')).toBeTruthy();

		const { rerender } = render(<StatusPill active={false} activeLabel="Listening" inactiveLabel="Stopped" />);
		rerender(<StatusPill active={false} activeLabel="Listening" inactiveLabel="Stopped" />);
		expect(screen.getByText('Stopped')).toBeTruthy();
	});

	it('applies correct type classes', () => {
		const { container: successContainer } = render(<StatusPill active={true} type="success" />);
		expect(successContainer.querySelector('.bg-statusBgSuccess')).toBeTruthy();

		const { container: errorContainer } = render(<StatusPill active={true} type="error" />);
		expect(errorContainer.querySelector('.bg-statusBgError')).toBeTruthy();

		const { container: warningContainer } = render(<StatusPill active={true} type="warning" />);
		expect(warningContainer.querySelector('.bg-statusBgWarning')).toBeTruthy();

		const { container: infoContainer } = render(<StatusPill active={true} type="info" />);
		expect(infoContainer.querySelector('.bg-statusBgInfo')).toBeTruthy();
	});

	it('shows dot indicator', () => {
		const { container } = render(<StatusPill active={true} />);
		const dot = container.querySelector('.rounded-full');
		expect(dot).toBeTruthy();
	});
});

