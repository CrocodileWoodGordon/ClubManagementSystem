import { SectionCard } from "@/components/common/SectionCard";

export default function TermSettingsLoading() {
    return (
        <div className="space-y-8">
            <SectionCard title="学期管理" description="正在加载学期信息...">
                <div className="space-y-4" aria-busy="true" aria-live="polite">
                    <div className="rounded-lg bg-slate-100 px-3 py-2 text-sm text-slate-500">
                        请稍候，正在获取全部学期数据。
                    </div>
                    <div className="space-y-3 rounded-xl border border-slate-200 p-4">
                        {[0, 1, 2].map((row) => (
                            <div key={row} className="h-10 w-full rounded-md bg-slate-100" />
                        ))}
                    </div>
                    <div className="space-y-3 rounded-xl border border-slate-200 p-4">
                        {[0, 1, 2, 3].map((row) => (
                            <div key={row} className="h-9 w-full rounded-md bg-slate-100" />
                        ))}
                    </div>
                </div>
            </SectionCard>
        </div>
    );
}
