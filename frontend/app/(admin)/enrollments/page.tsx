import { SectionCard } from "@/components/common/SectionCard";
import { EnrollmentImportPanel } from "@/components/upload/EnrollmentImportPanel";
import type { EnrollmentSummaryRow, PendingEnrollment } from "@/lib/types";
import { formatWeekday } from "@/lib/utils";
import {
    fetchEnrollmentSummary,
    fetchPendingEnrollments,
} from "@/services/enrollmentService";

const STATUS_LABELS: Record<string, string> = {
    PENDING: "待分班",
    ACTIVE: "已激活",
    DROPPED: "已退课",
    TRANSFERRED_OUT: "已转出",
    TRANSFERRED_IN: "转入待确认",
};

export default async function EnrollmentPage() {
    const [pendingEnrollments, summaryRows] = await Promise.all([
        fetchPendingEnrollments(),
        fetchEnrollmentSummary(),
    ]);

    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">报名管理</h1>
            <SectionCard
                title="导入问卷星报名"
                description="上传 Excel 后会即时解析并返回逐行处理结果"
            >
                <EnrollmentImportPanel />
            </SectionCard>
            <SectionCard title="报名汇总" description="按校区 / 社团 / 星期聚合的报名前端视图">
                <EnrollmentSummaryTable rows={summaryRows} />
            </SectionCard>
            <SectionCard title="待处理名单" description="解析成功但仍处于待分班状态的学生">
                <PendingEnrollmentTable rows={pendingEnrollments} />
            </SectionCard>
        </div>
    );
}

function EnrollmentSummaryTable({ rows }: { rows: EnrollmentSummaryRow[] }) {
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
                        <tr key={`${row.campusId}-${row.clubId}-${row.requestedWeekday}`} className="border-t">
                            <td className="px-4 py-2 text-slate-900">{row.campusName}</td>
                            <td className="px-4 py-2 text-slate-900">{row.clubName}</td>
                            <td className="px-4 py-2 text-slate-600">{formatWeekday(row.requestedWeekday)}</td>
                            <td className="px-4 py-2 font-semibold text-slate-900">{row.total}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

function PendingEnrollmentTable({ rows }: { rows: PendingEnrollment[] }) {
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
                            <td className="px-4 py-2 font-medium text-slate-900">{enrollment.studentName}</td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.homeroom}</td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.campusName}</td>
                            <td className="px-4 py-2 text-slate-600">{enrollment.clubName}</td>
                            <td className="px-4 py-2 text-slate-600">{formatWeekday(enrollment.requestedWeekday)}</td>
                            <td className="px-4 py-2 text-slate-600">{formatStatus(enrollment.status)}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

function formatStatus(status: PendingEnrollment["status"]) {
    return STATUS_LABELS[status] ?? status;
}
