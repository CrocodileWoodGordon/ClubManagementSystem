import { MetricCard } from "@/components/widgets/MetricCard";
import { SectionCard } from "@/components/common/SectionCard";

const metrics = [
    { label: "待分班人数", value: "150" },
    { label: "已建班级", value: "32" },
    { label: "待导入考勤", value: "12" },
];

export default function DashboardPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">工作台</h1>
            <div className="grid gap-4 sm:grid-cols-3">
                {metrics.map((metric) => (
                    <MetricCard key={metric.label} label={metric.label} value={metric.value} />
                ))}
            </div>
            <SectionCard title="下一步" description="引导管理员按顺序完成流程">
                <ol className="list-decimal space-y-2 pl-5 text-sm text-slate-600">
                    <li>上传问卷星 Excel 导入报名数据</li>
                    <li>按社团+星期批量分配班级编号</li>
                    <li>导出并发放考勤表，期末上传结果</li>
                    <li>触发结算计算并导出账单</li>
                </ol>
            </SectionCard>
        </div>
    );
}
