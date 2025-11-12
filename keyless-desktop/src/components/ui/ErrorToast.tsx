import { useEffect, useState, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../../utils/logger';

interface ErrorToastState {
	message: string;
	id: number;
}

/**
 * Error toast component that displays user-facing error messages.
 * Listens for error events from the backend and displays them as dismissible toasts.
 */
export function ErrorToast() {
	const [toast, setToast] = useState<ErrorToastState | null>(null);
	const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const idCounterRef = useRef(0);

	const showToast = useCallback((message: string, duration = 5000) => {
		// Clear any existing timer
		if (hideTimerRef.current) {
			clearTimeout(hideTimerRef.current);
			hideTimerRef.current = null;
		}

		// Show new toast
		const id = ++idCounterRef.current;
		setToast({
			message,
			id,
		});

		// Auto-dismiss after duration
		hideTimerRef.current = setTimeout(() => {
			setToast(null);
			hideTimerRef.current = null;
		}, duration);
	}, []);

	const handleDismiss = useCallback(() => {
		if (hideTimerRef.current) {
			clearTimeout(hideTimerRef.current);
			hideTimerRef.current = null;
		}
		setToast(null);
	}, []);

	// Listen for error events from backend
	useEffect(() => {
		let unsub: (() => void) | undefined;

		listen<{ message?: string }>('error_notification', (event) => {
			const payload = event.payload;
			const message = typeof payload === 'object' && payload?.message
				? payload.message
				: typeof payload === 'string'
					? payload
					: 'an error occurred';
			showToast(message);
		})
			.then((u) => {
				unsub = u;
			})
			.catch((error) => {
				logError('Failed to attach error_notification listener:', error);
			});

		return () => {
			if (unsub) unsub();
			if (hideTimerRef.current) {
				clearTimeout(hideTimerRef.current);
			}
		};
	}, [showToast]);

	if (!toast) {
		return null;
	}

	return (
		<div className="fixed bottom-4 right-4 z-50 max-w-md fade-in slide-up">
			<div className="mx-3 rounded-lg border border-errorDanger bg-bgCard px-4 py-3 shadow-lg">
				<div className="flex items-start justify-between gap-3">
					<div className="flex-1 min-w-0">
						<div className="text-[13px] font-medium text-errorTextAlt lowercase mb-1">
							error
						</div>
						<div className="text-[12px] text-textSecondary lowercase break-words">
							{toast.message}
						</div>
					</div>
					<button
						onClick={handleDismiss}
						aria-label="dismiss error"
						className="flex-shrink-0 text-textSecondary hover:text-textPrimary text-lg leading-none px-1 py-0.5 -mt-1 -mr-1"
					>
						×
					</button>
				</div>
			</div>
		</div>
	);
}

