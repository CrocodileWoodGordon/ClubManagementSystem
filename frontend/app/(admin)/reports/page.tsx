"use client";

import type { ChangeEvent } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { SectionCard } from "@/components/common/SectionCard";
import { MetricCard } from "@/components/widgets/MetricCard";
import type {
    FeeBreakdown,
    HomeroomRoster,
    RosterStudent,
} from "@/lib/types";
import { formatCurrency, formatWaiverReason } from "@/lib/utils";
import { exportCsv, exportHomeroomBillingExcel } from "@/lib/utils/export";
import { fetchHomeroomBilling, fetchStudentBilling } from "@/services/reportingService";
import {
    fetchHomeroomStudents,
    fetchHomerooms,
} from "@/services/studentRosterService";

interface Option {
    value: string;
    label: string;
}

const ALL_STUDENTS_VALUE = "__ALL__";

export default function ReportsPage() {
    const [homerooms, setHomerooms] = useState<HomeroomRoster[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        let active = true;
        fetchHomerooms()
            .then((rows) => {
                if (!active) {
                    return;
                }
                setHomerooms(rows);
                setError(null);
            })
            .catch((err) => {
                if (!active) {
                    return;
                }
                setError(err instanceof Error ? err.message : String(err));
            })
            .finally(() => {
                if (active) {
                    setLoading(false);
                }
            });

        return () => {
            active = false;
        };
    }, []);

    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">报表导出</h1>
            <SectionCard
                title="学生账单预览"
                description="选择班级和学生，实时汇总课时费/材料费，支持导出 CSV 发送给家长。"
            >
                {loading ? (
                    <LoadingHint />
                ) : error ? (
                    <ErrorHint message={error} />
                ) : homerooms.length === 0 ? (
                    <EmptyHint />
                ) : (
                    <ReportsWorkspace homerooms={homerooms} />
                )}
            </SectionCard>
        </div>
    );
}

function LoadingHint() {
    return (
        <div className="space-y-3">
            {[1, 2, 3].map((item) => (
                <div
                    key={item}
                    className="h-4 animate-pulse rounded-full bg-slate-200"
                    style={{ width: `${60 + item * 8}%` }}
                />
            ))}
        </div>
    );
}

function ErrorHint({ message }: { message: string }) {
    return <p className="text-sm text-red-600">加载学生名册失败：{message}</p>;
}

function EmptyHint() {
    return <p className="text-sm text-slate-500">暂无学生名册记录。</p>;
}

interface ReportsWorkspaceProps {
    homerooms: HomeroomRoster[];
}

function ReportsWorkspace({ homerooms }: ReportsWorkspaceProps) {
    const campusOptions = useMemo(() => {
        const seen = new Map<string, string>();
        homerooms.forEach((room) => {
            if (!seen.has(room.campusId)) {
                seen.set(room.campusId, room.campusName);
            }
        });
        return Array.from(seen.entries()).map(([value, label]) => ({ value, label }));
    }, [homerooms]);

    const firstCampus = homerooms[0]?.campusId ?? "";
    const firstHomeroom =
        homerooms.find((room) => room.campusId === firstCampus) ?? homerooms[0];

    const [selectedCampus, setSelectedCampus] = useState(firstCampus);
    const homeroomOptions = useMemo(() => {
        if (!selectedCampus) {
            return homerooms;
        }
        return homerooms.filter((room) => room.campusId === selectedCampus);
    }, [homerooms, selectedCampus]);

    const [selectedHomeroomId, setSelectedHomeroomId] = useState(firstHomeroom?.id ?? "");
    const [students, setStudents] = useState<RosterStudent[]>([]);
    const [studentsLoading, setStudentsLoading] = useState(false);
    const [studentsError, setStudentsError] = useState<string | null>(null);

    const [selectedStudentId, setSelectedStudentId] = useState("");
    const isAllStudentsSelected = selectedStudentId === ALL_STUDENTS_VALUE;
    const [billingRows, setBillingRows] = useState<FeeBreakdown[]>([]);
    const [billingLoading, setBillingLoading] = useState(false);
    const [billingError, setBillingError] = useState<string | null>(null);

    const studentOptions = useMemo(() => {
        if (students.length === 0) {
            return [] as Option[];
        }
        const mapped = students.map((student) => ({
            value: student.id,
            label: `${student.fullName}${student.studentCode ? `（${student.studentCode}）` : ""}`,
        }));
        return [{ value: ALL_STUDENTS_VALUE, label: "全部" }, ...mapped];
    }, [students]);

    const [exporting, setExporting] = useState(false);

    const resetStudentsState = useCallback(() => {
        setStudents([]);
        setStudentsError(null);
        setSelectedStudentId("");
    }, []);

    const resetBillingState = useCallback(() => {
        setBillingRows([]);
        setBillingError(null);
    }, []);

    const studentRequestRef = useRef(0);
    const billingRequestRef = useRef(0);

    const loadStudents = useCallback(async () => {
        if (!selectedHomeroomId) {
            return;
        }
        studentRequestRef.current += 1;
        const requestId = studentRequestRef.current;
        setStudentsLoading(true);
        setStudentsError(null);
        try {
            const rows = await fetchHomeroomStudents(selectedHomeroomId);
            if (studentRequestRef.current !== requestId) {
                return;
            }
            setStudents(rows);
            if (rows.length === 0) {
                setSelectedStudentId("");
                resetBillingState();
                return;
            }
            setSelectedStudentId((previous) => {
                if (previous === ALL_STUDENTS_VALUE) {
                    return previous;
                }
                if (previous && rows.some((student) => student.id === previous)) {
                    return previous;
                }
                resetBillingState();
                return rows[0].id;
            });
        } catch (err) {
            if (studentRequestRef.current !== requestId) {
                return;
            }
            setStudentsError(err instanceof Error ? err.message : String(err));
            setStudents([]);
            setSelectedStudentId("");
            resetBillingState();
        } finally {
            if (studentRequestRef.current === requestId) {
                setStudentsLoading(false);
            }
        }
    }, [selectedHomeroomId, resetBillingState]);

    const loadBilling = useCallback(async () => {
        if (!selectedStudentId) {
            return;
        }
        billingRequestRef.current += 1;
        const requestId = billingRequestRef.current;
        setBillingLoading(true);
        setBillingError(null);
        try {
            const rows = await fetchStudentBilling(selectedStudentId);
            if (billingRequestRef.current !== requestId) {
                return;
            }
            setBillingRows(rows);
        } catch (err) {
            if (billingRequestRef.current !== requestId) {
                return;
            }
            setBillingRows([]);
            setBillingError(err instanceof Error ? err.message : String(err));
        } finally {
            if (billingRequestRef.current === requestId) {
                setBillingLoading(false);
            }
        }
    }, [selectedStudentId]);

    const selectedHomeroom = useMemo(
        () => homerooms.find((room) => room.id === selectedHomeroomId),
        [homerooms, selectedHomeroomId],
    );

    const selectedStudent = useMemo(() => {
        if (isAllStudentsSelected || !selectedStudentId) {
            return undefined;
        }
        return students.find((student) => student.id === selectedStudentId);
    }, [students, selectedStudentId, isAllStudentsSelected]);

    const totals = useMemo(() => {
        return billingRows.reduce(
            (acc, row) => {
                acc.lesson += row.lessonFee;
                acc.material += row.materialFee;
                acc.discount += row.discountAmount;
                acc.charged += row.chargedSessions;
                return acc;
            },
            { lesson: 0, material: 0, discount: 0, charged: 0 },
        );
    }, [billingRows]);

    const canExportHomeroom =
        isAllStudentsSelected && Boolean(selectedHomeroomId) && students.length > 0;
    const canExportStudent = !isAllStudentsSelected && billingRows.length > 0;
    const exportDisabled = exporting || (!canExportHomeroom && !canExportStudent);
    const exportLabel = exporting
        ? "生成中..."
        : isAllStudentsSelected
          ? "导出 Excel（整班）"
          : "导出 CSV";

    useEffect(() => {
        if (!selectedHomeroomId) {
            return;
        }
        loadStudents();
    }, [selectedHomeroomId, loadStudents]);

    useEffect(() => {
        if (!selectedStudentId || isAllStudentsSelected) {
            return;
        }
        loadBilling();
    }, [selectedStudentId, isAllStudentsSelected, loadBilling]);

    const handleCampusChange = (event: ChangeEvent<HTMLSelectElement>) => {
        const campusId = event.target.value;
        setSelectedCampus(campusId);
        const nextHomeroom =
            homerooms.find((room) => room.campusId === campusId) ?? homerooms[0];
        setSelectedHomeroomId(nextHomeroom?.id ?? "");
        resetStudentsState();
        resetBillingState();
    };

    const handleHomeroomChange = (event: ChangeEvent<HTMLSelectElement>) => {
        setSelectedHomeroomId(event.target.value);
        resetStudentsState();
        resetBillingState();
    };

    const handleStudentChange = (event: ChangeEvent<HTMLSelectElement>) => {
        setSelectedStudentId(event.target.value);
        resetBillingState();
    };

    const handleExportSingle = () => {
        if (billingRows.length === 0 || !selectedStudent) {
            return;
        }
        const header = [
            "班级ID",
            "出勤次数",
            "计费课次",
            "课时费",
            "材料费",
            "减免金额",
            "退课/优惠原因",
            "备注",
        ];
        const csvRows = billingRows.map((row) => [
            row.classId,
            String(row.attendanceCount),
            String(row.chargedSessions),
            row.lessonFee.toFixed(2),
            row.materialFee.toFixed(2),
            row.discountAmount.toFixed(2),
            formatWaiverReason(row.waiveReason),
            row.remarks ?? "",
        ]);
        const name = selectedStudent.fullName || "student";
        exportCsv(header, csvRows, `billing_${name}.csv`);
    };

    const handleHomeroomExport = async () => {
        if (!selectedHomeroomId) {
            setBillingError("请选择班级");
            return;
        }
        setExporting(true);
        setBillingError(null);
        try {
            const report = await fetchHomeroomBilling(selectedHomeroomId);
            if (report.students.length === 0) {
                setBillingError("该班暂无学生账单记录，无法导出。");
                return;
            }
            const dateLabel = new Date().toISOString().slice(0, 10);
            const campus = report.homeroom.campusName || "campus";
            const homeroomLabel = report.homeroom.displayName || "class";
            const fileName = `${homeroomLabel}_${campus}_${dateLabel}.xls`;
            exportHomeroomBillingExcel(report, fileName);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setBillingError(message);
        } finally {
            setExporting(false);
        }
    };

    const handleExport = async () => {
        if (isAllStudentsSelected) {
            await handleHomeroomExport();
            return;
        }
        if (billingRows.length === 0 || !selectedStudent) {
            return;
        }
        setExporting(true);
        try {
            handleExportSingle();
        } finally {
            setExporting(false);
        }
    };

    return (
        <div className="space-y-6">
            <div className="grid gap-4 md:grid-cols-3">
                <FilterSelect
                    label="校区"
                    value={selectedCampus}
                    options={campusOptions}
                    onChange={handleCampusChange}
                />
                <FilterSelect
                    label="班级"
                    value={selectedHomeroomId}
                    options={homeroomOptions.map((room) => ({
                        value: room.id,
                        label: `${room.displayName}（${room.studentCount}人）`,
                    }))}
                    placeholder={studentsLoading ? "加载中..." : "请选择班级"}
                    onChange={handleHomeroomChange}
                />
                <FilterSelect
                    label="学生"
                    value={selectedStudentId}
                    options={studentOptions}
                    placeholder={
                        studentsLoading ? "学生加载中..." : "请选择学生"
                    }
                    onChange={handleStudentChange}
                />
            </div>

            {studentsError ? (
                <p className="text-sm text-red-600">学生列表加载失败：{studentsError}</p>
            ) : null}
            {billingError ? (
                <p className="text-sm text-red-600">账单加载失败：{billingError}</p>
            ) : null}
            {studentsLoading ? (
                <p className="text-sm text-slate-500">正在加载学生名单...</p>
            ) : null}
            {billingLoading ? (
                <p className="text-sm text-slate-500">正在计算账单...</p>
            ) : null}

            {isAllStudentsSelected ? (
                <div className="rounded-xl border border-dashed border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-800">
                    已选择导出整班账单，列表预览暂停显示。请直接点击下方“导出 Excel（整班）”按钮生成文件。
                </div>
            ) : selectedStudent ? (
                <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-600">
                    <p>
                        当前学生：<span className="font-semibold text-slate-900">{selectedStudent.fullName}</span>
                        {selectedStudent.studentCode
                            ? `（学号 ${selectedStudent.studentCode}）`
                            : ""}
                    </p>
                    {selectedHomeroom ? (
                        <p>
                            所属班级：{selectedHomeroom.displayName} · 校区{" "}
                            {selectedHomeroom.campusName}
                        </p>
                    ) : null}
                </div>
            ) : (
                <p className="text-sm text-slate-500">请选择学生以查看账单。</p>
            )}

            <div className="flex flex-wrap items-center gap-3">
                {isAllStudentsSelected ? (
                    <div className="flex-1 rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
                        整班导出模式下不展示个人汇总。
                    </div>
                ) : (
                    <div className="grid flex-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
                        <MetricCard label="课时费" value={formatCurrency(totals.lesson)} />
                        <MetricCard label="材料费" value={formatCurrency(totals.material)} />
                        <MetricCard label="课时费减免" value={formatCurrency(totals.discount)} />
                        <MetricCard label="计费课次" value={totals.charged.toString()} />
                    </div>
                )}
                <button
                    type="button"
                    className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-60"
                    onClick={() => {
                        void handleExport();
                    }}
                    disabled={exportDisabled}
                >
                    {exportLabel}
                </button>
            </div>

            {isAllStudentsSelected ? (
                <div className="rounded-xl border border-dashed border-slate-300 bg-white p-6 text-sm text-slate-500">
                    整班导出将直接在生成的 Excel 中查看每位学生的账单明细。
                </div>
            ) : (
                <div className="overflow-x-auto rounded-xl border border-slate-200">
                    {billingRows.length === 0 ? (
                        <div className="p-6 text-sm text-slate-500">
                            暂无账单行。
                        </div>
                    ) : (
                        <table className="min-w-full divide-y divide-slate-200 text-sm">
                            <thead className="bg-slate-50">
                                <tr>
                                    <th className="px-4 py-2 text-left font-medium text-slate-600">
                                        班级 ID
                                    </th>
                                    <th className="px-4 py-2 text-left font-medium text-slate-600">
                                        出勤/计费
                                    </th>
                                    <th className="px-4 py-2 text-right font-medium text-slate-600">
                                        课时费
                                    </th>
                                    <th className="px-4 py-2 text-right font-medium text-slate-600">
                                        材料费
                                    </th>
                                    <th className="px-4 py-2 text-right font-medium text-slate-600">
                                        减免
                                    </th>
                                    <th className="px-4 py-2 text-left font-medium text-slate-600">
                                        原因
                                    </th>
                                    <th className="px-4 py-2 text-left font-medium text-slate-600">
                                        备注
                                    </th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-slate-100 bg-white">
                                {billingRows.map((row) => (
                                    <tr key={`${row.classId}-${row.enrollmentId}`}>
                                        <td className="px-4 py-3 text-slate-900">
                                            {row.classId}
                                        </td>
                                        <td className="px-4 py-3 text-slate-700">
                                            {row.attendanceCount} 次 / {row.chargedSessions} 课次
                                        </td>
                                        <td className="px-4 py-3 text-right font-semibold text-slate-900">
                                            {formatCurrency(row.lessonFee)}
                                        </td>
                                        <td className="px-4 py-3 text-right font-semibold text-slate-900">
                                            {formatCurrency(row.materialFee)}
                                        </td>
                                        <td className="px-4 py-3 text-right text-emerald-700">
                                            {formatCurrency(row.discountAmount)}
                                        </td>
                                        <td className="px-4 py-3 text-slate-700">
                                            {formatWaiverReason(row.waiveReason)}
                                        </td>
                                        <td className="px-4 py-3 text-slate-500">
                                            {row.remarks ?? "—"}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    )}
                </div>
            )}
        </div>
    );
}

interface FilterSelectProps {
    label: string;
    value: string;
    options: Option[];
    onChange: (event: ChangeEvent<HTMLSelectElement>) => void;
    placeholder?: string;
}

function FilterSelect({
    label,
    value,
    options,
    onChange,
    placeholder = "请选择",
}: FilterSelectProps) {
    return (
        <label className="flex flex-col gap-1 text-sm text-slate-600">
            {label}
            <select
                className="rounded-lg border border-slate-300 px-3 py-2 text-sm text-slate-900 focus:border-slate-500 focus:outline-none"
                value={value}
                onChange={onChange}
            >
                {options.length === 0 ? (
                    <option value="">{placeholder}</option>
                ) : null}
                {options.map((option) => (
                    <option key={option.value} value={option.value}>
                        {option.label}
                    </option>
                ))}
            </select>
        </label>
    );
}
