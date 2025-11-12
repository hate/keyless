import { Component, type ReactNode } from 'react';
import { logError } from '../utils/logger';

interface ErrorBoundaryProps {
	children: ReactNode;
	fallback?: ReactNode;
}

interface ErrorBoundaryState {
	hasError: boolean;
	error?: Error;
}

/**
 * Error Boundary component to catch and handle React component errors.
 * Prevents the entire app from crashing when a component throws an error.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
	constructor(props: ErrorBoundaryProps) {
		super(props);
		this.state = { hasError: false };
	}

	static getDerivedStateFromError(error: Error): ErrorBoundaryState {
		return { hasError: true, error };
	}

	componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
		logError('ErrorBoundary caught an error:', error, errorInfo);
	}

	handleReset = () => {
		this.setState({ hasError: false, error: undefined });
	};

	render() {
		if (this.state.hasError) {
			if (this.props.fallback) {
				return this.props.fallback;
			}

			return (
				<div className="mx-3 mb-2 rounded-xl border border-errorDanger bg-bgCard p-8 text-center">
					<p className="text-errorTextAlt lowercase mb-2">something went wrong</p>
					{this.state.error && (
						<p className="text-textSecondary text-[11px] lowercase mb-4 font-mono">
							{this.state.error.message}
						</p>
					)}
					<button
						onClick={this.handleReset}
						className="px-4 py-2 rounded border border-border bg-bgInput text-textPrimary hover:border-textPrimary btn-anim"
					>
						try again
					</button>
				</div>
			);
		}

		return this.props.children;
	}
}

