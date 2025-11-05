import { BrandTitle } from '../components/BrandTitle';
import { AudioVisualizer } from '../components/AudioVisualizer';

export { Arrow } from './Idle';

export default function ListeningView({ bars, transcript, onOpenSettings }: { bars: number[]; transcript: string; onOpenSettings?: () => void }) {
  return (
    <>
      <div className="mx-3 mb-3 rounded-xl border border-[#2a2a2a] bg-[#0f0f0f] min-h-[240px] flex flex-col py-[30px] px-[22px]">
        <div className="mb-6">
          <BrandTitle />
        </div>
        <div className="flex-1 w-full space-y-6">
          <div className="text-center">
            <div className="text-[36px] font-bold text-[#e5e5e5] tracking-[0.05em] uppercase">LISTENING</div>
          </div>
          <AudioVisualizer bars={bars} />
          <div className="px-4">
            <p className="text-sm text-[#e5e5e5] leading-relaxed text-center">{transcript}</p>
          </div>
        </div>
        <div className="mt-6 text-center">
          <button onClick={onOpenSettings} className="text-xs text-[#a0a0a0] hover:text-[#e5e5e5] transition-colors lowercase bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none">[settings]</button>
        </div>
      </div>
    </>
  );
}


