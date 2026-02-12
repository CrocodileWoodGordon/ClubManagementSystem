export default function AttendanceLoading() {
    return (
        <div className="space-y-4">
            {[1, 2, 3].map((item) => (
                <div
                    key={item}
                    className="h-12 animate-pulse rounded-2xl bg-slate-200/80"
                />
            ))}
        </div>
    );
}
