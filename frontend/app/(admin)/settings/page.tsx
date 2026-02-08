import Link from "next/link";

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
            <SectionCard
                title="占位文本管理"
                description="维护 Excel 导入时代表“跳过/空”的字符串，统一作用于报名与学生导入流程。"
            >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <p className="text-sm text-slate-600">
                        点击下方按钮可查看当前占位字符串、进行新增/删除并立即生效。
                    </p>
                    <Link
                        href="/settings/placeholders"
                        className="inline-flex items-center justify-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-indigo-500"
                    >
                        管理占位文本
                    </Link>
                </div>
            </SectionCard>
        </div>
    );
}
