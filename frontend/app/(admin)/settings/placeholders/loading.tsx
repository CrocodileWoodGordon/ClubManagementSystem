import { SectionCard } from "@/components/common/SectionCard";

export default function EnrollmentPlaceholderLoading() {
    return (
        <div className="space-y-8">
            <SectionCard
                title="占位文本设置"
                description="正在加载占位字符串..."
            >
                <div className="space-y-4" aria-busy="true" aria-live="polite">
                    <div className="rounded-lg bg-slate-100 px-3 py-2 text-sm text-slate-500">
                        请稍候，正在读取占位文本配置。
                    </div>
                    <div className="space-y-4">
                        {[0, 1].map((section) => (
                            <div key={section} className="space-y-3 rounded-lg border border-slate-200 p-4">
                                <div className="h-4 w-48 rounded bg-slate-200" />
                                <div className="space-y-2">
                                    {[0, 1, 2].map((row) => (
                                        <div key={row} className="h-8 w-full rounded bg-slate-100" />
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </SectionCard>
        </div>
    );
}
