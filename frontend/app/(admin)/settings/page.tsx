import { SectionCard } from "@/components/common/SectionCard";

export default function SettingsPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">系统设置</h1>
            <SectionCard title="费用参数" description="材料费、课时费、退课规则">
                <p className="text-sm text-slate-600">
                    未来在此配置每个社团的材料费和课时费，供后端计算使用。
                </p>
            </SectionCard>
            <SectionCard title="基础数据" description="班级、老师、教室等基础信息维护">
                <p className="text-sm text-slate-600">用于 Excel 解析和排课约束。</p>
            </SectionCard>
        </div>
    );
}
