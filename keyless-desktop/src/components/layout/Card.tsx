import { ReactNode } from 'react';

export interface CardProps {
    children: ReactNode;
    className?: string;
}

/**
 * A reusable card wrapper component for consistent styling across views.
 * Applies standard card styling (background, border, rounded corners, margins).
 */
export function Card({ children, className = '' }: CardProps) {
    return (
        <div className={`mx-3 mb-2 rounded-xl border border-border bg-bgCard ${className}`}>
            {children}
        </div>
    );
}

