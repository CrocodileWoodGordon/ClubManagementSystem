interface MetricCardProps {
    label: string;
    value: string;
    hint?: string;
}

export function MetricCard({ label, value, hint }: MetricCardProps) {
    return (
        <div className="rounded-xl border border-slate-200 bg-white px-4 py-3">
            <p className="text-sm text-slate-500">{label}</p>
            <p className="text-2xl font-semibold text-slate-900">{value}</p>
            {hint ? <p className="text-xs text-slate-400">{hint}</p> : null}
        </div>
    );
}
