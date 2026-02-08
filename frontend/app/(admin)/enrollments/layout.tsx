import type { ReactNode } from "react";

import { EnrollmentTabNav } from "@/components/enrollments/EnrollmentTabNav";

const ENROLLMENT_TABS = [
    { href: "/enrollments/import", label: "导入名单" },
    { href: "/enrollments/summary", label: "报名汇总" },
    { href: "/enrollments/slots", label: "筛选报名名单" },
    { href: "/enrollments/pending", label: "待处理名单" },
];

export default function EnrollmentLayout({ children }: { children: ReactNode }) {
    return (
        <div className="space-y-6">
            <div className="space-y-2">
                <h1 className="text-2xl font-semibold text-slate-900">报名管理</h1>
                <p className="text-sm text-slate-600">
                    根据任务切换子页面，只有在进入相关功能时才会加载对应数据。
                </p>
            </div>
            <EnrollmentTabNav tabs={ENROLLMENT_TABS} />
            <div className="space-y-8">{children}</div>
        </div>
    );
}
