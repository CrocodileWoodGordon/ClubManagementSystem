export default function ReportsLoading() {
    return (
        <div className="space-y-4">
            {[1, 2, 3].map((item) => (
                <div
                    key={item}
                    className="h-10 animate-pulse rounded-2xl bg-slate-200/80"
                />
            ))}
        </div>
    );
}
