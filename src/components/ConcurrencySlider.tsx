interface ConcurrencySliderProps {
  value: number;
  onChange: (val: number) => void;
  disabled?: boolean;
}

export default function ConcurrencySlider({
  value,
  onChange,
  disabled = false,
}: ConcurrencySliderProps) {
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        min={1}
        max={50}
        step={1}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-32 h-1.5 bg-white/10 rounded-full appearance-none cursor-pointer accent-blue-500 disabled:opacity-40 disabled:cursor-not-allowed [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-blue-500"
      />
      <span className="text-xs text-gray-400 whitespace-nowrap min-w-[60px]">
        {value} thread{value !== 1 ? "s" : ""}
      </span>
    </div>
  );
}
