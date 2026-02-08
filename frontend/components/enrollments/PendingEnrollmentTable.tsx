import type { PendingEnrollment } from "@/lib/types";
import { formatEnrollmentStatus, formatWeekday } from "@/lib/utils";

interface Props {
    rows: PendingEnrollment[];
}

export function PendingEnrollmentTable({ rows }: Props) {
    if (rows.length === 0) {
        return <p className="text-sm text-slate-500">暂无待分班学生。</p>;
    }

    return (
        <div className="overflow-x-auto">
            <table className="min-w-full rounded-xl border border-slate-200 text-sm">
                <thead className="bg-slate-50 text-left text-slate-500">
                    <tr>
                        <th className="px-4 py-2">学生姓名</th>
                        <th className="px-4 py-2">所属班级</th>
                        <th className="px-4 py-2">校区</th>
                        <th className="px-4 py-2">社团</th>
                        <th className="px-4 py-2">星期</th>
                        <th className="px-4 py-2">状态</th>
                    </tr>
                </thead>
                <tbody>
                    {rows.map((enrollment) => (
                        <tr key={enrollment.enrollmentId} className="border-t">
                            <td className="px-4 py-2 font-medium text-slate-900">
                                {enrollment.studentName}
                            </td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.homeroom}</td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.campusName}</td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.clubName}</td>
                            <td className="px-4 py-2 text-slate-600">
                                {formatWeekday(enrollment.requestedWeekday)}
                            </td>
                            <td className="px-4 py-2 text-slate-600">
                                {formatEnrollmentStatus(enrollment.status)}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
