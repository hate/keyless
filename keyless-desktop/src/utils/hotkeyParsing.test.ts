import { describe, it, expect } from 'vitest';
import { parseHotkeyToPttListener } from './hotkeyParsing';

describe('parseHotkeyToPttListener', () => {
	it('parses Control+Shift variants to ctrlShift', () => {
		expect(parseHotkeyToPttListener('Control+Shift')).toBe('ctrlShift');
		expect(parseHotkeyToPttListener('control+shift')).toBe('ctrlShift');
		expect(parseHotkeyToPttListener('Ctrl+Shift')).toBe('ctrlShift');
		expect(parseHotkeyToPttListener('ctrl+shift')).toBe('ctrlShift');
		expect(parseHotkeyToPttListener('CONTROL+SHIFT')).toBe('ctrlShift');
	});

	it('defaults to ctrlOption for other hotkeys', () => {
		expect(parseHotkeyToPttListener('Control+Option')).toBe('ctrlOption');
		expect(parseHotkeyToPttListener('Ctrl+Alt')).toBe('ctrlOption');
		expect(parseHotkeyToPttListener('anything else')).toBe('ctrlOption');
		expect(parseHotkeyToPttListener('')).toBe('ctrlOption');
	});
});

