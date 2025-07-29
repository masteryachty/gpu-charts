import { useAppStore } from '../../store/useAppStore';

export default function StatusBar() {
  const { symbol } = useAppStore();

  return (
    <div className="h-10 bg-bg-secondary border-t border-border flex items-center justify-between px-6 text-sm">
      {symbol}
    </div>
  );
}