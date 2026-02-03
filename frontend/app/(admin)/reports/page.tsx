import { SectionCard } from "@/components/common/SectionCard";

export default function ReportsPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">报表导出</h1>
            <SectionCard title="班级报表" description="学生名单、考勤率、费用明细">
                <p className="text-sm text-slate-600">支持一键导出为 Excel 或 PDF。</p>
            </SectionCard>
            <SectionCard title="个人账单" description="每位学生的上课、退课、费用记录">
                <p className="text-sm text-slate-600">可同时生成老师子女名单供财务审核。</p>
            </SectionCard>
        </div>
    );
}
