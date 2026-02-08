"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

interface TabLink {
    href: string;
    label: string;
}

export function EnrollmentTabNav({ tabs }: { tabs: TabLink[] }) {
    const pathname = usePathname();

    return (
        <div className="border-b border-slate-200">
            <nav className="flex flex-wrap gap-2" aria-label="Enrollment sections">
                {tabs.map((tab) => {
                    const isActive =
                        pathname === tab.href || pathname.startsWith(`${tab.href}/`);
                    return (
                        <Link
                            key={tab.href}
                            href={tab.href}
                            aria-current={isActive ? "page" : undefined}
                            className={[
                                "rounded-t-lg border-b-2 px-4 py-2 text-sm font-medium transition",
                                isActive
                                    ? "border-indigo-500 bg-white text-indigo-600 shadow"
                                    : "border-transparent text-slate-600 hover:border-slate-300 hover:text-slate-900",
                            ].join(" ")}
                        >
                            {tab.label}
                        </Link>
                    );
                })}
            </nav>
        </div>
    );
}
