import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { IconButton } from './IconButton';

describe('IconButton', () => {
    it('renders with icon and label', () => {
        const handleClick = vi.fn();
        render(<IconButton icon="download" label="Download" onClick={handleClick} />);

        const button = screen.getByRole('button', { name: 'Download' });
        expect(button).toBeTruthy();
        expect(button.getAttribute('title')).toBe('Download');
        expect(button.getAttribute('aria-label')).toBe('Download');
    });

    it('calls onClick when clicked', async () => {
        const user = userEvent.setup();
        const handleClick = vi.fn();
        render(<IconButton icon="download" label="Download" onClick={handleClick} />);

        const button = screen.getByRole('button');
        await user.click(button);
        expect(handleClick).toHaveBeenCalledTimes(1);
    });

    it('does not call onClick when disabled', async () => {
        const user = userEvent.setup();
        const handleClick = vi.fn();
        render(<IconButton icon="download" label="Download" onClick={handleClick} disabled />);

        const button = screen.getByRole('button');
        expect(button.hasAttribute('disabled')).toBe(true);
        await user.click(button);
        expect(handleClick).not.toHaveBeenCalled();
    });

    it('shows label text when showLabel is true', () => {
        render(<IconButton icon="download" label="Download" onClick={vi.fn()} showLabel />);
        expect(screen.getByText('Download')).toBeTruthy();
    });

    it('applies variant classes', () => {
        const { container: primary } = render(<IconButton icon="download" label="Download" onClick={vi.fn()} variant="primary" />);
        expect(primary.querySelector('.bg-textPrimary')).toBeTruthy();

        const { container: secondary } = render(<IconButton icon="download" label="Download" onClick={vi.fn()} variant="secondary" />);
        expect(secondary.querySelector('.bg-bgInput')).toBeTruthy();

        const { container: danger } = render(<IconButton icon="delete" label="Delete" onClick={vi.fn()} variant="danger" />);
        expect(danger.querySelector('.text-errorDanger')).toBeTruthy();
    });

    it('calls onMouseEnter and onMouseLeave', async () => {
        const user = userEvent.setup();
        const handleMouseEnter = vi.fn();
        const handleMouseLeave = vi.fn();
        render(
            <IconButton
                icon="download"
                label="Download"
                onClick={vi.fn()}
                onMouseEnter={handleMouseEnter}
                onMouseLeave={handleMouseLeave}
            />
        );

        const button = screen.getByRole('button');
        await user.hover(button);
        expect(handleMouseEnter).toHaveBeenCalledTimes(1);

        await user.unhover(button);
        expect(handleMouseLeave).toHaveBeenCalledTimes(1);
    });
});

