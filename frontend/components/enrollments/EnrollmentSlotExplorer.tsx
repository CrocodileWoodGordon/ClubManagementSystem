"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { EnrollmentSummaryRow, PendingEnrollment } from "@/lib/types";
import { formatEnrollmentStatus, formatWeekday } from "@/lib/utils";
import { fetchEnrollmentSlotDetails } from "@/services/enrollmentService";

interface Option {
    value: string;
    label: string;
}

interface Props {
    summaryRows: EnrollmentSummaryRow[];
}

export function EnrollmentSlotExplorer({ summaryRows }: Props) {
    const [selectedCampus, setSelectedCampus] = useState(
        () => summaryRows[0]?.campusId ?? "",
    );
    const [selectedClub, setSelectedClub] = useState(() => summaryRows[0]?.clubId ?? "");
    const [selectedWeekday, setSelectedWeekday] = useState(() =>
        summaryRows[0] ? String(summaryRows[0].requestedWeekday) : "",
    );

    const campusOptions = useMemo(() => dedupeCampuses(summaryRows), [summaryRows]);
    const clubOptions = useMemo(
        () => dedupeClubs(summaryRows, selectedCampus),
        [summaryRows, selectedCampus],
    );
    const weekdayOptions = useMemo(
        () => collectWeekdays(summaryRows, selectedCampus, selectedClub),
        [summaryRows, selectedCampus, selectedClub],
    );

    const handleCampusChange = (value: string) => {
        setSelectedCampus(value);
        const nextClubs = dedupeClubs(summaryRows, value);
        const nextClubId = nextClubs[0]?.value ?? "";
        setSelectedClub(nextClubId);
        const nextWeekdays = collectWeekdays(summaryRows, value, nextClubId);
        setSelectedWeekday(nextWeekdays[0]?.value ?? "");
    };

    const handleClubChange = (value: string) => {
        setSelectedClub(value);
        const nextWeekdays = collectWeekdays(summaryRows, selectedCampus, value);
        setSelectedWeekday(nextWeekdays[0]?.value ?? "");
    };

    const handleWeekdayChange = (value: string) => {
        setSelectedWeekday(value);
    };

    const [students, setStudents] = useState<PendingEnrollment[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const requestIdRef = useRef(0);

    const loadStudents = useCallback(
        async (campusId: string, clubId: string, weekday: number) => {
            const requestId = requestIdRef.current + 1;
            requestIdRef.current = requestId;
            setLoading(true);
            setError(null);
            try {
                const result = await fetchEnrollmentSlotDetails({
                    campusId,
                    clubId,
                    weekday,
                });
                if (requestId === requestIdRef.current) {
                    setStudents(result);
                }
            } catch (err) {
                if (requestId === requestIdRef.current) {
                    const message = err instanceof Error ? err.message : String(err);
                    setError(message);
                    setStudents([]);
                }
            } finally {
                if (requestId === requestIdRef.current) {
                    setLoading(false);
                }
            }
        },
        [],
    );

    const hasSelection = Boolean(selectedCampus && selectedClub && selectedWeekday);

    useEffect(() => {
        if (!hasSelection) {
            return;
        }
        loadStudents(selectedCampus, selectedClub, Number(selectedWeekday));
    }, [hasSelection, loadStudents, selectedCampus, selectedClub, selectedWeekday]);

    if (summaryRows.length === 0) {
        return null;
    }

    const selectionSummary = summaryRows.find(
        (row) =>
            row.campusId === selectedCampus &&
            row.clubId === selectedClub &&
            String(row.requestedWeekday) === selectedWeekday,
    );

    return (
        <div className="space-y-4 rounded-xl border border-slate-200 p-4">
            <div className="space-y-2">
                <h4 className="text-base font-semibold text-slate-900">筛选报名名单</h4>
                <p className="text-sm text-slate-500">
                    选择校区 / 社团 / 星期后，将调用实时接口加载该组合下的报名学生。
                </p>
            </div>
            <div className="grid gap-4 md:grid-cols-3">
                <FilterControl
                    label="校区"
                    value={selectedCampus}
                    onChange={handleCampusChange}
                    options={campusOptions}
                    placeholder="选择校区"
                />
                <FilterControl
                    label="社团"
                    value={selectedClub}
                    onChange={handleClubChange}
                    options={clubOptions}
                    placeholder="选择社团"
                />
                <FilterControl
                    label="星期"
                    value={selectedWeekday}
                    onChange={handleWeekdayChange}
                    options={weekdayOptions}
                    placeholder="选择星期"
                />
            </div>
            <div className="rounded-lg bg-slate-50 p-3 text-sm text-slate-600">
                {selectedCampus && selectedClub && selectedWeekday ? (
                    <span>
                        当前组合：{selectionSummary?.campusName ?? "--"} /{" "}
                        {selectionSummary?.clubName ?? "--"} /{" "}
                        {formatWeekday(Number(selectedWeekday))}（报名 {selectionSummary?.total ?? 0} 人）
                    </span>
                ) : (
                    <span>请完成全部筛选条件以查看名单。</span>
                )}
            </div>
            <SlotResult
                loading={loading}
                error={error}
                students={students}
                hasSelection={hasSelection}
            />
        </div>
    );
}

function FilterControl({
    label,
    value,
    onChange,
    options,
    placeholder,
}: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    options: Option[];
    placeholder: string;
}) {
    return (
        <label className="flex flex-col gap-2 text-sm text-slate-600">
            <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">{label}</span>
            <select
                className="rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-100"
                value={value}
                onChange={(event) => onChange(event.target.value)}
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

function SlotResult({
    loading,
    error,
    students,
    hasSelection,
}: {
    loading: boolean;
    error: string | null;
    students: PendingEnrollment[];
    hasSelection: boolean;
}) {
    if (!hasSelection) {
        return <p className="text-sm text-slate-500">请先完成筛选条件。</p>;
    }

    if (loading) {
        return <p className="text-sm text-slate-500">正在加载报名名单...</p>;
    }

    if (error) {
        return (
            <p className="text-sm text-rose-600">
                加载失败：{error}，请稍后重试。
            </p>
        );
    }

    if (students.length === 0) {
        return <p className="text-sm text-slate-500">暂无报名学生。</p>;
    }

    return (
        <div className="overflow-x-auto">
            <table className="min-w-full rounded-xl border border-slate-200 text-sm">
                <thead className="bg-white text-left text-slate-500">
                    <tr>
                        <th className="px-4 py-2">学生姓名</th>
                        <th className="px-4 py-2">所属班级</th>
                        <th className="px-4 py-2">学号/编号</th>
                        <th className="px-4 py-2">状态</th>
                    </tr>
                </thead>
                <tbody>
                    {students.map((student) => (
                        <tr key={student.enrollmentId} className="border-t">
                            <td className="px-4 py-2 font-semibold text-slate-900">{student.studentName}</td>
                            <td className="px-4 py-2 text-slate-600">{student.homeroom}</td>
                            <td className="px-4 py-2 text-slate-600">{student.studentCode ?? "--"}</td>
                            <td className="px-4 py-2 text-slate-600">
                                {formatEnrollmentStatus(student.status)}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

function dedupeCampuses(rows: EnrollmentSummaryRow[]): Option[] {
    const map = new Map<string, Option>();
    rows.forEach((row) => {
        if (!map.has(row.campusId)) {
            map.set(row.campusId, { value: row.campusId, label: row.campusName });
        }
    });
    return Array.from(map.values()).sort((a, b) => a.label.localeCompare(b.label, "zh-Hans"));
}

function dedupeClubs(rows: EnrollmentSummaryRow[], campusId: string): Option[] {
    const filtered = campusId ? rows.filter((row) => row.campusId === campusId) : rows;
    const map = new Map<string, Option>();
    filtered.forEach((row) => {
        if (!map.has(row.clubId)) {
            map.set(row.clubId, { value: row.clubId, label: row.clubName });
        }
    });
    return Array.from(map.values()).sort((a, b) => a.label.localeCompare(b.label, "zh-Hans"));
}

function collectWeekdays(
    rows: EnrollmentSummaryRow[],
    campusId: string,
    clubId: string,
): Option[] {
    const filtered = rows.filter((row) => {
        if (campusId && row.campusId !== campusId) {
            return false;
        }
        if (clubId && row.clubId !== clubId) {
            return false;
        }
        return true;
    });
    const map = new Map<string, Option>();
    filtered.forEach((row) => {
        const key = String(row.requestedWeekday);
        if (!map.has(key)) {
            map.set(key, { value: key, label: formatWeekday(row.requestedWeekday) });
        }
    });
    return Array.from(map.values()).sort(
        (a, b) => Number(a.value) - Number(b.value),
    );
}
