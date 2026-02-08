import { SectionCard } from "@/components/common/SectionCard";

const PLACEHOLDER_ROWS = Array.from({ length: 5 });

export default function PendingEnrollmentLoading() {
    return (
        <div className="space-y-8">
            <SectionCard
                title="待处理名单"
                description="正在加载待分班学生，请稍候..."
            >
                <div className="space-y-4" aria-busy="true" aria-live="polite">
                    <div className="rounded-lg bg-slate-100 px-3 py-2 text-sm text-slate-500">
                        系统正在查询最新名单，页面加载完成前保持此状态。
                    </div>
                    <div className="overflow-x-auto">
                        <table className="min-w-full rounded-xl border border-slate-200 text-sm">
                            <thead className="bg-slate-50 text-left text-slate-500">
                                <tr>
                                    <th className="px-4 py-2">学生姓名</th>
                                    <th className="px-4 py-2">所属班级</th>
                                    <th className="px-4 py-2">校区</th>
                                    <th className="px-4 py-2">社团</th>
                                    <th className="px-4 py-2">星期</th>
                                    <th className="px-4 py-2">状态</th>
                                </tr>
                            </thead>
                            <tbody>
                                {PLACEHOLDER_ROWS.map((_, index) => (
                                    <tr key={index} className="border-t animate-pulse">
                                        <SkeletonCell />
                                        <SkeletonCell />
                                        <SkeletonCell />
                                        <SkeletonCell />
                                        <SkeletonCell />
                                        <SkeletonCell />
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            </SectionCard>
        </div>
    );
}

function SkeletonCell() {
    return (
        <td className="px-4 py-3">
            <div className="h-3.5 w-full rounded bg-slate-200" />
        </td>
    );
}
