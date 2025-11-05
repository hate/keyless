import { BrandTitle } from '../components/BrandTitle';

export function Arrow() {
    return (
        <div className="w-full h-[14px] relative -mb-px">
            <div className="absolute top-0 left-1/2 -translate-x-1/2 w-0 h-0 border-l-[9px] border-l-transparent border-r-[9px] border-r-transparent border-b-[14px] border-b-[#2a2a2a]" />
            <div className="absolute top-px left-1/2 -translate-x-1/2 w-0 h-0 border-l-[8px] border-l-transparent border-r-[8px] border-r-transparent border-b-[13px] border-b-[#0f0f0f]" />
        </div>
    );
}

export default function IdleView({ hotkey, onOpenSettings }: { hotkey?: string; onOpenSettings?: () => void }) {
    return (
        <>
            <div className="mx-3 mb-3 rounded-xl border border-[#2a2a2a] bg-[#0f0f0f] min-h-[240px] flex flex-col py-[30px] px-[22px]">
                <div className="mb-6">
                    <BrandTitle />
                </div>
                <div className="flex-1 flex flex-col items-center justify-center text-center space-y-[30px]">
                    <div className="text-[36px] font-bold text-[#e5e5e5] tracking-[0.05em] uppercase">IDLE</div>
                    <p className="text-xs text-[#a0a0a0] lowercase">
                        hold <span className="bg-[#2a2a2a] text-[#e5e5e5] px-2 py-0.5 rounded font-medium">{hotkey || 'control+option'}</span> to start listening
                    </p>
                </div>
                <div className="mt-6 text-center">
                    <button
                        onClick={onOpenSettings}
                        className="text-xs text-[#a0a0a0] hover:text-[#e5e5e5] transition-colors lowercase bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none"
                    >
                        [settings]
                    </button>
                </div>
            </div>
        </>
    );
}


