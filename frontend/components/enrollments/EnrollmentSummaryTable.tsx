import type { EnrollmentSummaryRow } from "@/lib/types";
import { formatWeekday } from "@/lib/utils";

interface Props {
    rows: EnrollmentSummaryRow[];
}

export function EnrollmentSummaryTable({ rows }: Props) {
    if (rows.length === 0) {
        return <p className="text-sm text-slate-500">暂无报名数据。</p>;
    }

    return (
        <div className="overflow-x-auto">
            <table className="min-w-full rounded-xl border border-slate-200 text-sm">
                <thead className="bg-slate-50 text-left text-slate-500">
                    <tr>
                        <th className="px-4 py-2">校区</th>
                        <th className="px-4 py-2">社团</th>
                        <th className="px-4 py-2">星期</th>
                        <th className="px-4 py-2">报名人数</th>
                    </tr>
                </thead>
                <tbody>
                    {rows.map((row) => (
                        <tr
                            key={`${row.campusId}-${row.clubId}-${row.requestedWeekday}`}
                            className="border-t"
                        >
                            <td className="px-4 py-2 text-slate-900">{row.campusName}</td>
                            <td className="px-4 py-2 text-slate-900">{row.clubName}</td>
                            <td className="px-4 py-2 text-slate-600">
                                {formatWeekday(row.requestedWeekday)}
                            </td>
                            <td className="px-4 py-2 font-semibold text-slate-900">{row.total}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
