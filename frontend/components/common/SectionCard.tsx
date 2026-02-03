import type { ReactNode } from "react";

interface SectionCardProps {
    title: string;
    description?: string;
    children: ReactNode;
}

export function SectionCard({ title, description, children }: SectionCardProps) {
    return (
        <section className="bg-white rounded-2xl border border-slate-200 shadow-sm p-6 space-y-3">
            <div>
                <h2 className="text-lg font-semibold text-slate-900">{title}</h2>
                {description ? (
                    <p className="text-sm text-slate-500">{description}</p>
                ) : null}
            </div>
            <div>{children}</div>
        </section>
    );
}
