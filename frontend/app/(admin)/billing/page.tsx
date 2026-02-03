import { SectionCard } from "@/components/common/SectionCard";

export default function BillingPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">费用结算</h1>
            <SectionCard title="结算预览" description="根据考勤生成课时费和材料费明细">
                <p className="text-sm text-slate-600">
                    先运行预览，确认教师子女免课时费、三节内退课规则，再锁定数据。
                </p>
            </SectionCard>
            <SectionCard title="明细导出" description="按班级/学生导出 Excel">
                <p className="text-sm text-slate-600">生成账单后可下载给家长签字确认。</p>
            </SectionCard>
        </div>
    );
}
