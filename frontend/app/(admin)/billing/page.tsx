"use client";

import type { ChangeEvent } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { SectionCard } from "@/components/common/SectionCard";
import { MetricCard } from "@/components/widgets/MetricCard";
import type {
    ClassInstance,
    EnrollmentSummaryRow,
    FeeBreakdown,
    PendingEnrollment,
} from "@/lib/types";
import { formatCurrency, formatWaiverReason, formatWeekday } from "@/lib/utils";
import { exportCsv } from "@/lib/utils/export";
import { fetchClassesForSlot } from "@/services/classAssignmentService";
import {
    fetchEnrollmentSlotDetails,
    fetchEnrollmentSummary,
} from "@/services/enrollmentService";
import { fetchClassSettlement } from "@/services/reportingService";

interface Option {
    value: string;
    label: string;
}

export default function BillingPage() {
    const [summaryRows, setSummaryRows] = useState<EnrollmentSummaryRow[]>([]);
    const [summaryLoading, setSummaryLoading] = useState(true);
    const [summaryError, setSummaryError] = useState<string | null>(null);

    useEffect(() => {
        let active = true;
        fetchEnrollmentSummary()
            .then((rows) => {
                if (!active) {
                    return;
                }
                setSummaryRows(rows);
                setSummaryError(null);
            })
            .catch((error) => {
                if (!active) {
                    return;
                }
                setSummaryError(error instanceof Error ? error.message : String(error));
            })
            .finally(() => {
                if (active) {
                    setSummaryLoading(false);
                }
            });

        return () => {
            active = false;
        };
    }, []);

    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">费用结算</h1>
            <SectionCard
                title="结算预览工作台"
                description="按校区 / 社团 / 星期选择班级，实时查看课时费、材料费与减免明细，并可导出 CSV。"
            >
                {summaryLoading ? (
                    <LoadingHint />
                ) : summaryError ? (
                    <ErrorHint message={summaryError} />
                ) : summaryRows.length === 0 ? (
                    <EmptyHint />
                ) : (
                    <BillingWorkspace summaryRows={summaryRows} />
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
                    style={{ width: `${50 + item * 10}%` }}
                />
            ))}
        </div>
    );
}

function ErrorHint({ message }: { message: string }) {
    return (
        <p className="text-sm text-red-600">
            加载报名汇总失败：{message}
        </p>
    );
}

function EmptyHint() {
    return <p className="text-sm text-slate-500">暂无报名数据，无法预览结算。</p>;
}

interface BillingWorkspaceProps {
    summaryRows: EnrollmentSummaryRow[];
}

function BillingWorkspace({ summaryRows }: BillingWorkspaceProps) {
    const firstRow = summaryRows[0];
    const campusOptions = useMemo(() => dedupeCampuses(summaryRows), [summaryRows]);
    const [selectedCampus, setSelectedCampus] = useState(firstRow?.campusId ?? "");
    const clubOptions = useMemo(
        () => dedupeClubs(summaryRows, selectedCampus),
        [summaryRows, selectedCampus],
    );
    const [selectedClub, setSelectedClub] = useState(firstRow?.clubId ?? "");
    const weekdayOptions = useMemo(
        () => collectWeekdays(summaryRows, selectedCampus, selectedClub),
        [summaryRows, selectedCampus, selectedClub],
    );
    const [selectedWeekday, setSelectedWeekday] = useState(
        firstRow ? String(firstRow.requestedWeekday) : "",
    );
    const selectedWeekdayNumber = selectedWeekday ? Number(selectedWeekday) : null;
    const canLoadSlot = Boolean(selectedCampus && selectedClub && selectedWeekdayNumber);

    const [classes, setClasses] = useState<ClassInstance[]>([]);
    const [classLoading, setClassLoading] = useState(false);
    const [classError, setClassError] = useState<string | null>(null);
    const [selectedClassId, setSelectedClassId] = useState("");

    const [slotStudents, setSlotStudents] = useState<PendingEnrollment[]>([]);
    const [slotLoading, setSlotLoading] = useState(false);
    const [slotError, setSlotError] = useState<string | null>(null);

    const [settlementRows, setSettlementRows] = useState<FeeBreakdown[]>([]);
    const [settlementLoading, setSettlementLoading] = useState(false);
    const [settlementError, setSettlementError] = useState<string | null>(null);

    const [exporting, setExporting] = useState(false);

    const resetClassState = useCallback(() => {
        setClasses([]);
        setClassError(null);
        setSelectedClassId("");
    }, []);

    const resetSlotState = useCallback(() => {
        setSlotStudents([]);
        setSlotError(null);
    }, []);

    const resetSettlementState = useCallback(() => {
        setSettlementRows([]);
        setSettlementError(null);
    }, []);

    const resetWorkspaceState = useCallback(() => {
        resetClassState();
        resetSlotState();
        resetSettlementState();
    }, [resetClassState, resetSlotState, resetSettlementState]);

    const classRequestRef = useRef(0);
    const slotRequestRef = useRef(0);
    const settlementRequestRef = useRef(0);

    const loadClassData = useCallback(async () => {
        if (!canLoadSlot || !selectedWeekdayNumber) {
            return;
        }
        classRequestRef.current += 1;
        const requestId = classRequestRef.current;
        setClassLoading(true);
        setClassError(null);
        try {
            const data = await fetchClassesForSlot({
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
            });
            if (classRequestRef.current !== requestId) {
                return;
            }
            setClasses(data);
            if (data.length === 0) {
                setSelectedClassId("");
                resetSettlementState();
                return;
            }
            setSelectedClassId((previous) => {
                if (previous && data.some((cls) => cls.id === previous)) {
                    return previous;
                }
                resetSettlementState();
                return data[0].id;
            });
        } catch (error) {
            if (classRequestRef.current !== requestId) {
                return;
            }
            setClassError(error instanceof Error ? error.message : String(error));
            setClasses([]);
            setSelectedClassId("");
            resetSettlementState();
        } finally {
            if (classRequestRef.current === requestId) {
                setClassLoading(false);
            }
        }
    }, [canLoadSlot, selectedCampus, selectedClub, selectedWeekdayNumber, resetSettlementState]);

    const loadSlotData = useCallback(async () => {
        if (!canLoadSlot || !selectedWeekdayNumber) {
            return;
        }
        slotRequestRef.current += 1;
        const requestId = slotRequestRef.current;
        setSlotLoading(true);
        setSlotError(null);
        try {
            const data = await fetchEnrollmentSlotDetails({
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
            });
            if (slotRequestRef.current !== requestId) {
                return;
            }
            setSlotStudents(data);
        } catch (error) {
            if (slotRequestRef.current !== requestId) {
                return;
            }
            setSlotError(error instanceof Error ? error.message : String(error));
            setSlotStudents([]);
        } finally {
            if (slotRequestRef.current === requestId) {
                setSlotLoading(false);
            }
        }
    }, [canLoadSlot, selectedCampus, selectedClub, selectedWeekdayNumber]);

    const loadSettlementData = useCallback(async () => {
        if (!selectedClassId) {
            return;
        }
        settlementRequestRef.current += 1;
        const requestId = settlementRequestRef.current;
        setSettlementLoading(true);
        setSettlementError(null);
        try {
            const rows = await fetchClassSettlement(selectedClassId);
            if (settlementRequestRef.current !== requestId) {
                return;
            }
            setSettlementRows(rows);
        } catch (error) {
            if (settlementRequestRef.current !== requestId) {
                return;
            }
            setSettlementError(error instanceof Error ? error.message : String(error));
            setSettlementRows([]);
        } finally {
            if (settlementRequestRef.current === requestId) {
                setSettlementLoading(false);
            }
        }
    }, [selectedClassId]);

    const studentLookup = useMemo(() => {
        const map = new Map<string, PendingEnrollment>();
        slotStudents.forEach((student) => map.set(student.enrollmentId, student));
        return map;
    }, [slotStudents]);

    const selectedClass = useMemo(
        () => classes.find((cls) => cls.id === selectedClassId),
        [classes, selectedClassId],
    );

    const totals = useMemo(() => {
        return settlementRows.reduce(
            (acc, row) => {
                acc.lesson += row.lessonFee;
                acc.material += row.materialFee;
                acc.discount += row.discountAmount;
                acc.charged += row.chargedSessions;
                return acc;
            },
            { lesson: 0, material: 0, discount: 0, charged: 0 },
        );
    }, [settlementRows]);

    const enrichedRows = useMemo(() => {
        return settlementRows.map((row) => {
            const student = studentLookup.get(row.enrollmentId);
            return {
                ...row,
                studentName: student?.studentName ?? "未知学生",
                studentCode: student?.studentCode,
                status: student?.status,
            };
        });
    }, [settlementRows, studentLookup]);


    useEffect(() => {
        if (!canLoadSlot || !selectedWeekdayNumber) {
            return;
        }
        loadClassData();
    }, [canLoadSlot, selectedWeekdayNumber, loadClassData]);

    useEffect(() => {
        if (!canLoadSlot || !selectedWeekdayNumber) {
            return;
        }
        loadSlotData();
    }, [canLoadSlot, selectedWeekdayNumber, loadSlotData]);

    useEffect(() => {
        if (!selectedClassId) {
            return;
        }
        loadSettlementData();
    }, [selectedClassId, loadSettlementData]);

    const handleCampusChange = (event: ChangeEvent<HTMLSelectElement>) => {
        const campusId = event.target.value;
        setSelectedCampus(campusId);
        const nextClubId = dedupeClubs(summaryRows, campusId)[0]?.value ?? "";
        setSelectedClub(nextClubId);
        const nextWeekday =
            collectWeekdays(summaryRows, campusId, nextClubId)[0]?.value ?? "";
        setSelectedWeekday(nextWeekday);
        resetWorkspaceState();
    };

    const handleClubChange = (event: ChangeEvent<HTMLSelectElement>) => {
        const clubId = event.target.value;
        setSelectedClub(clubId);
        const nextWeekday =
            collectWeekdays(summaryRows, selectedCampus, clubId)[0]?.value ?? "";
        setSelectedWeekday(nextWeekday);
        resetWorkspaceState();
    };

    const handleWeekdayChange = (event: ChangeEvent<HTMLSelectElement>) => {
        setSelectedWeekday(event.target.value);
        resetWorkspaceState();
    };

    const handleClassChange = (event: ChangeEvent<HTMLSelectElement>) => {
        setSelectedClassId(event.target.value);
        resetSettlementState();
    };

    const handleExport = () => {
        if (settlementRows.length === 0) {
            return;
        }
        setExporting(true);
        const header = [
            "学生姓名",
            "学号/编号",
            "出勤次数",
            "计费课次",
            "课时费",
            "材料费",
            "减免金额",
            "退课/优惠原因",
            "备注",
        ];
        const csvRows = settlementRows.map((row) => {
            const student = studentLookup.get(row.enrollmentId);
            return [
                student?.studentName ?? "",
                student?.studentCode ?? "",
                String(row.attendanceCount),
                String(row.chargedSessions),
                row.lessonFee.toFixed(2),
                row.materialFee.toFixed(2),
                row.discountAmount.toFixed(2),
                formatWaiverReason(row.waiveReason),
                row.remarks ?? "",
            ];
        });
        exportCsv(
            header,
            csvRows,
            `settlement_${selectedClass?.classCode ?? selectedClassId}.csv`,
        );
        setExporting(false);
    };

    return (
        <div className="space-y-6">
            <div className="grid gap-4 md:grid-cols-4">
                <FilterSelect
                    label="校区"
                    value={selectedCampus}
                    options={campusOptions}
                    onChange={handleCampusChange}
                />
                <FilterSelect
                    label="社团"
                    value={selectedClub}
                    options={clubOptions}
                    onChange={handleClubChange}
                />
                <FilterSelect
                    label="星期"
                    value={selectedWeekday}
                    options={weekdayOptions}
                    onChange={handleWeekdayChange}
                />
                <FilterSelect
                    label="班级"
                    value={selectedClassId}
                    options={classes.map((cls) => ({
                        value: cls.id,
                        label: `${cls.classCode}（${formatWeekday(cls.weekday)} ${cls.startTime}-${cls.endTime}）`,
                    }))}
                    placeholder={classLoading ? "加载中..." : "请选择班级"}
                    onChange={handleClassChange}
                />
            </div>

            {classError ? (
                <p className="text-sm text-red-600">班级加载失败：{classError}</p>
            ) : null}
            {slotError ? (
                <p className="text-sm text-amber-600">报名名单加载失败：{slotError}</p>
            ) : null}
            {classLoading ? (
                <p className="text-sm text-slate-500">正在读取班级列表...</p>
            ) : null}
            {slotLoading ? (
                <p className="text-sm text-slate-500">正在读取报名名单...</p>
            ) : null}

            {selectedClass ? (
                <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-600">
                    <p>
                        当前班级：<span className="font-semibold text-slate-900">{selectedClass.classCode}</span>{" "}
                        · {formatWeekday(selectedClass.weekday)} {selectedClass.startTime} -{" "}
                        {selectedClass.endTime} （已分班 {selectedClass.assignedCount}
                        人{selectedClass.capacity ? ` / 容量 ${selectedClass.capacity}` : ""}）
                    </p>
                </div>
            ) : (
                <p className="text-sm text-slate-500">请选择班级以查看结算。</p>
            )}

            <div className="flex flex-wrap items-center gap-3">
                <div className="grid flex-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
                    <MetricCard label="课时费" value={formatCurrency(totals.lesson)} />
                    <MetricCard label="材料费" value={formatCurrency(totals.material)} />
                    <MetricCard label="课时费减免" value={formatCurrency(totals.discount)} />
                    <MetricCard
                        label="计费课次"
                        value={totals.charged.toString()}
                        hint="来自出勤记录"
                    />
                </div>
                <button
                    type="button"
                    className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-60"
                    onClick={handleExport}
                    disabled={settlementRows.length === 0 || exporting}
                >
                    {exporting ? "生成中..." : "导出 CSV"}
                </button>
            </div>

            <div className="overflow-x-auto rounded-xl border border-slate-200">
                {settlementLoading ? (
                    <div className="p-6 text-sm text-slate-500">结算数据加载中...</div>
                ) : settlementError ? (
                    <div className="p-6 text-sm text-red-600">加载失败：{settlementError}</div>
                ) : settlementRows.length === 0 ? (
                    <div className="p-6 text-sm text-slate-500">
                        暂无结算行，请确认考勤与报名数据。
                    </div>
                ) : (
                    <table className="min-w-full divide-y divide-slate-200 text-sm">
                        <thead className="bg-slate-50">
                            <tr>
                                <th className="px-4 py-2 text-left font-medium text-slate-600">
                                    学生
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
                            {enrichedRows.map((row) => (
                                <tr key={row.enrollmentId}>
                                    <td className="px-4 py-3">
                                        <p className="font-medium text-slate-900">
                                            {row.studentName}
                                        </p>
                                        <p className="text-xs text-slate-500">
                                            {row.studentCode ?? row.studentId}
                                        </p>
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

function dedupeCampuses(rows: EnrollmentSummaryRow[]): Option[] {
    const seen = new Map<string, string>();
    rows.forEach((row) => {
        if (!seen.has(row.campusId)) {
            seen.set(row.campusId, row.campusName);
        }
    });
    return Array.from(seen.entries()).map(([value, label]) => ({ value, label }));
}

function dedupeClubs(rows: EnrollmentSummaryRow[], campusId: string): Option[] {
    if (!campusId) {
        return [];
    }
    const filtered = rows.filter((row) => row.campusId === campusId);
    const seen = new Map<string, string>();
    filtered.forEach((row) => {
        if (!seen.has(row.clubId)) {
            seen.set(row.clubId, row.clubName);
        }
    });
    return Array.from(seen.entries()).map(([value, label]) => ({ value, label }));
}

function collectWeekdays(
    rows: EnrollmentSummaryRow[],
    campusId: string,
    clubId: string,
): Option[] {
    if (!campusId || !clubId) {
        return [];
    }
    const filtered = rows.filter(
        (row) => row.campusId === campusId && row.clubId === clubId,
    );
    const seen = new Set<number>();
    filtered.forEach((row) => seen.add(row.requestedWeekday));
    return Array.from(seen.values())
        .sort((a, b) => a - b)
        .map((weekday) => ({
            value: String(weekday),
            label: `${formatWeekday(weekday)}（${filtered.find((row) => row.requestedWeekday === weekday)?.total ?? 0}人）`,
        }));
}
