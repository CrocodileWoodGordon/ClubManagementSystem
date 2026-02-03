import Link from "next/link";
import type { ReactNode } from "react";

const NAV_LINKS = [
    { href: "/dashboard", label: "仪表盘" },
    { href: "/enrollments", label: "报名管理" },
    { href: "/class-assignment", label: "分班工具" },
    { href: "/attendance", label: "考勤导入" },
    { href: "/billing", label: "费用结算" },
    { href: "/reports", label: "报表导出" },
    { href: "/students", label: "学生名册" },
    { href: "/settings", label: "系统设置" },
];

export default function AdminLayout({ children }: { children: ReactNode }) {
    return (
        <div className="min-h-screen flex bg-slate-50 text-slate-900">
            <aside className="w-64 border-r border-slate-200 bg-white">
                <div className="px-6 py-5 font-semibold text-lg">社团管理后台</div>
                <nav className="flex flex-col gap-1 px-4">
                    {NAV_LINKS.map((link) => (
                        <Link
                            key={link.href}
                            href={link.href}
                            className="rounded-lg px-3 py-2 text-sm font-medium text-slate-600 hover:bg-slate-100"
                        >
                            {link.label}
                        </Link>
                    ))}
                </nav>
            </aside>
            <main className="flex-1 p-8">{children}</main>
        </div>
    );
}
