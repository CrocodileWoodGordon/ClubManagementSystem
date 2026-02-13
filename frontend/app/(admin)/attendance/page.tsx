"use client";

import type { ChangeEvent, FormEvent, ReactNode } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { SectionCard } from "@/components/common/SectionCard";
import type {
    AttendanceImportResult,
    AttendanceMeeting,
    AttendanceRecord,
    ClassInstance,
    EnrollmentSummaryRow,
} from "@/lib/types";
import { formatAttendanceStatus, formatWeekday } from "@/lib/utils";
import { downloadBase64File } from "@/lib/utils/export";
import { fetchEnrollmentSummary } from "@/services/enrollmentService";
import { fetchClassesForSlot } from "@/services/classAssignmentService";
import {
    fetchAttendanceHistory,
    fetchAttendanceTemplate,
    importAttendanceRecords,
} from "@/services/attendanceService";

export default function AttendancePage() {
    const [summaryRows, setSummaryRows] = useState<EnrollmentSummaryRow[]>([]);
    const [loadingSummary, setLoadingSummary] = useState(true);
    const [summaryError, setSummaryError] = useState<string | null>(null);

    useEffect(() => {
        let mounted = true;
        fetchEnrollmentSummary()
            .then((rows) => {
                if (!mounted) {
                    return;
                }
                setSummaryRows(rows);
                setSummaryError(null);
            })
            .catch((error) => {
                if (!mounted) {
                    return;
                }
                const message = error instanceof Error ? error.message : String(error);
                setSummaryError(message);
            })
            .finally(() => {
                if (!mounted) {
                    return;
                }
                setLoadingSummary(false);
            });
        return () => {
            mounted = false;
        };
    }, []);

    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">考勤管理</h1>
            <SectionCard
                title="考勤导出 / 导入工作台"
                description="按校区 / 社团 / 星期筛选班级，下载模板、导入 Excel 并查看历史记录。"
            >
                {loadingSummary ? (
                    <LoadingHint />
                ) : summaryError ? (
                    <ErrorHint message={summaryError} />
                ) : (
                    <AttendanceWorkspace summaryRows={summaryRows} />
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
                    style={{ width: `${60 + item * 10}%` }}
                />
            ))}
        </div>
    );
}

function ErrorHint({ message }: { message: string }) {
    return (
        <p className="text-sm text-red-600">加载报名汇总失败：{message}，请稍后重试。</p>
    );
}

interface AttendanceWorkspaceProps {
    summaryRows: EnrollmentSummaryRow[];
}

function AttendanceWorkspace({ summaryRows }: AttendanceWorkspaceProps) {
    const campusOptions = useMemo(() => dedupeCampuses(summaryRows), [summaryRows]);
    const [selectedCampus, setSelectedCampus] = useState("");
    const clubOptions = useMemo(
        () => dedupeClubs(summaryRows, selectedCampus),
        [summaryRows, selectedCampus],
    );
    const [selectedClub, setSelectedClub] = useState("");
    const weekdayOptions = useMemo(
        () => collectWeekdays(summaryRows, selectedCampus, selectedClub),
        [summaryRows, selectedCampus, selectedClub],
    );
    const [selectedWeekday, setSelectedWeekday] = useState("");

    const [classes, setClasses] = useState<ClassInstance[]>([]);
    const [classLoading, setClassLoading] = useState(false);
    const [classError, setClassError] = useState<string | null>(null);
    const [selectedClassId, setSelectedClassId] = useState("");

    const [template, setTemplate] = useState<AttendanceTemplateState | null>(null);
    const [templateLoading, setTemplateLoading] = useState(false);
    const [templateError, setTemplateError] = useState<string | null>(null);
    const [selectedMeetingId, setSelectedMeetingId] = useState("");
    const [startWeek, setStartWeek] = useState("1");
    const [endWeek, setEndWeek] = useState("18");

    const [history, setHistory] = useState<AttendanceRecord[]>([]);
    const [historyLoading, setHistoryLoading] = useState(false);
    const [historyError, setHistoryError] = useState<string | null>(null);

    const [selectedFile, setSelectedFile] = useState<File | null>(null);
    const [recordedBy, setRecordedBy] = useState("");
    const [ignoredIdentifiers, setIgnoredIdentifiers] = useState("");
    const [uploading, setUploading] = useState(false);
    const [uploadError, setUploadError] = useState<string | null>(null);
    const [uploadResult, setUploadResult] = useState<AttendanceImportResult | null>(null);

    const hasFilters = Boolean(selectedCampus && selectedClub && selectedWeekday);
    const selectedWeekdayNumber = selectedWeekday ? Number(selectedWeekday) : null;

    useEffect(() => {
        if (!selectedCampus && campusOptions.length > 0) {
            setSelectedCampus(campusOptions[0].value);
        }
    }, [campusOptions, selectedCampus]);

    useEffect(() => {
        if (clubOptions.length === 0) {
            setSelectedClub("");
            return;
        }
        if (!clubOptions.some((option) => option.value === selectedClub)) {
            setSelectedClub(clubOptions[0].value);
        }
    }, [clubOptions, selectedClub]);

    useEffect(() => {
        if (weekdayOptions.length === 0) {
            setSelectedWeekday("");
            return;
        }
        if (!weekdayOptions.some((option) => option.value === selectedWeekday)) {
            setSelectedWeekday(weekdayOptions[0].value);
        }
    }, [weekdayOptions, selectedWeekday]);

    const resetWorkspaceState = useCallback(() => {
        setClasses([]);
        setClassError(null);
        setSelectedClassId("");
        setTemplate(null);
        setTemplateError(null);
        setSelectedMeetingId("");
        setStartWeek("1");
        setEndWeek("18");
        setHistory([]);
        setHistoryError(null);
        setSelectedFile(null);
        setUploadError(null);
        setUploadResult(null);
    }, []);

    useEffect(() => {
        resetWorkspaceState();
    }, [selectedCampus, selectedClub, selectedWeekday, resetWorkspaceState]);

    useEffect(() => {
        if (classes.length === 0) {
            setSelectedClassId("");
            return;
        }
        if (!classes.some((cls) => cls.id === selectedClassId)) {
            setSelectedClassId(classes[0].id);
        }
    }, [classes, selectedClassId]);

    useEffect(() => {
        setTemplate(null);
        setTemplateError(null);
        setSelectedMeetingId("");
        setHistory([]);
        setHistoryError(null);
        setUploadResult(null);
        setUploadError(null);
        setSelectedFile(null);
    }, [selectedClassId]);

    const selectedClass = useMemo(
        () => classes.find((cls) => cls.id === selectedClassId),
        [classes, selectedClassId],
    );

    const handleQueryClasses = async () => {
        if (!hasFilters || selectedWeekdayNumber === null) {
            setClassError("请先选择校区 / 社团 / 星期");
            return;
        }
        setClassLoading(true);
        setClassError(null);
        try {
            const slotClasses = await fetchClassesForSlot({
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
            });
            const sorted = [...slotClasses].sort((a, b) =>
                a.classCode.localeCompare(b.classCode, "zh-CN"),
            );
            setClasses(sorted);
            if (sorted.length === 0) {
                setSelectedClassId("");
            }
        } catch (error) {
            setClasses([]);
            setSelectedClassId("");
            const message = error instanceof Error ? error.message : String(error);
            setClassError(message);
        } finally {
            setClassLoading(false);
        }
    };

    const handleDownloadTemplate = async () => {
        if (!selectedClassId) {
            setTemplateError("请选择具体班级");
            return;
        }
        const trimmedStart = startWeek.trim();
        const trimmedEnd = endWeek.trim();
        const parsedStart =
            trimmedStart.length > 0 ? Number.parseInt(trimmedStart, 10) : undefined;
        if (trimmedStart.length > 0 && Number.isNaN(parsedStart)) {
            setTemplateError("起始周需为数字");
            return;
        }
        const parsedEnd = trimmedEnd.length > 0 ? Number.parseInt(trimmedEnd, 10) : undefined;
        if (trimmedEnd.length > 0 && Number.isNaN(parsedEnd)) {
            setTemplateError("终止周需为数字");
            return;
        }
        const MIN_WEEK = 1;
        const MAX_WEEK = 18;
        if (parsedStart !== undefined && (parsedStart < MIN_WEEK || parsedStart > MAX_WEEK)) {
            setTemplateError(`起始周需介于 ${MIN_WEEK}~${MAX_WEEK}`);
            return;
        }
        if (parsedEnd !== undefined && (parsedEnd < MIN_WEEK || parsedEnd > MAX_WEEK)) {
            setTemplateError(`终止周需介于 ${MIN_WEEK}~${MAX_WEEK}`);
            return;
        }
        if (parsedStart !== undefined && parsedEnd !== undefined && parsedStart > parsedEnd) {
            setTemplateError("起始周不能大于终止周");
            return;
        }
        setTemplateLoading(true);
        setTemplateError(null);
        try {
            const data = await fetchAttendanceTemplate(selectedClassId, {
                startWeek: parsedStart,
                endWeek: parsedEnd,
            });
            setTemplate({
                meetings: data.meetings,
                worksheetName: data.worksheet.name,
                rowCount: data.worksheet.rows.length,
                fileName: data.worksheet.fileName,
                fileBase64: data.worksheet.fileBase64,
                mimeType: data.worksheet.mimeType,
            });
            const firstMeeting = data.meetings[0]?.id ?? "";
            setSelectedMeetingId(firstMeeting);
            downloadBase64File(data.worksheet.fileBase64, data.worksheet.mimeType, data.worksheet.fileName);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setTemplateError(message);
        } finally {
            setTemplateLoading(false);
        }
    };

    const handleHistoryRefresh = async () => {
        if (!selectedClassId) {
            setHistoryError("请选择班级后再查询记录");
            return;
        }
        setHistoryLoading(true);
        setHistoryError(null);
        try {
            const records = await fetchAttendanceHistory({
                classId: selectedClassId,
                classMeetingId: selectedMeetingId || undefined,
            });
            setHistory(records);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setHistoryError(message);
            setHistory([]);
        } finally {
            setHistoryLoading(false);
        }
    };

    const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0] ?? null;
        setSelectedFile(file);
        event.target.value = "";
    };

    const handleUpload = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        if (!selectedClassId || !selectedMeetingId) {
            setUploadError("请先选择班级和课次");
            return;
        }
        if (!selectedFile) {
            setUploadError("请先上传 Excel 文件");
            return;
        }
        setUploading(true);
        setUploadError(null);
        try {
            const ignoredList = parseIdentifierInput(ignoredIdentifiers);
            const result = await importAttendanceRecords({
                classMeetingId: selectedMeetingId,
                file: selectedFile,
                recordedBy: recordedBy.trim() || undefined,
                ignoredIdentifiers: ignoredList,
            });
            setUploadResult(result);
            setSelectedFile(null);
            await handleHistoryRefresh();
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setUploadError(message);
            setUploadResult(null);
        } finally {
            setUploading(false);
        }
    };

    if (summaryRows.length === 0) {
        return (
            <p className="text-sm text-slate-500">暂无报名汇总数据，请先完成导入与分班。</p>
        );
    }

    return (
        <div className="space-y-6">
            <div className="grid gap-4 md:grid-cols-3">
                <Field label="校区">
                    <select
                        className="input"
                        value={selectedCampus}
                        onChange={(event) => setSelectedCampus(event.target.value)}
                    >
                        {campusOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                                {option.label}
                            </option>
                        ))}
                    </select>
                </Field>
                <Field label="社团">
                    <select
                        className="input"
                        value={selectedClub}
                        onChange={(event) => setSelectedClub(event.target.value)}
                    >
                        {clubOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                                {option.label}
                            </option>
                        ))}
                    </select>
                </Field>
                <Field label="星期">
                    <select
                        className="input"
                        value={selectedWeekday}
                        onChange={(event) => setSelectedWeekday(event.target.value)}
                    >
                        {weekdayOptions.map((option) => (
                            <option key={option.value} value={option.value}>
                                {formatWeekday(Number(option.value))}
                            </option>
                        ))}
                    </select>
                </Field>
            </div>

            <div className="flex flex-wrap items-center gap-3">
                <button
                    type="button"
                    className="btn-primary"
                    onClick={handleQueryClasses}
                    disabled={classLoading}
                >
                    {classLoading ? "加载中..." : "查询班级"}
                </button>
                {classError ? <p className="text-sm text-red-600">{classError}</p> : null}
            </div>

            {classes.length > 0 ? (
                <div className="space-y-3 rounded-2xl border border-slate-200 p-4">
                    <div className="grid gap-4 md:grid-cols-2">
                        <Field label="班级">
                            <select
                                className="input"
                                value={selectedClassId}
                                onChange={(event) => setSelectedClassId(event.target.value)}
                            >
                                {classes.map((cls) => (
                                    <option key={cls.id} value={cls.id}>
                                        {cls.classCode}（{cls.startTime}-{cls.endTime}）
                                    </option>
                                ))}
                            </select>
                        </Field>
                        {selectedClass ? (
                            <div className="text-sm text-slate-600">
                                <p>
                                    上课时间：{formatWeekday(selectedClass.weekday)} {selectedClass.startTime} - {selectedClass.endTime}
                                </p>
                                <p className="mt-1">
                                    地点：{selectedClass.location ?? "待定"} | 已分班 {selectedClass.assignedCount} 人
                                </p>
                            </div>
                        ) : null}
                    </div>
                    <div className="grid gap-4 md:grid-cols-2">
                        <Field label="模板起始周">
                            <input
                                className="input"
                                type="number"
                                min={1}
                                max={18}
                                step={1}
                                value={startWeek}
                                disabled={!selectedClassId}
                                onChange={(event) => {
                                    setStartWeek(event.target.value);
                                    setTemplateError(null);
                                }}
                            />
                        </Field>
                        <Field label="模板终止周">
                            <input
                                className="input"
                                type="number"
                                min={1}
                                max={18}
                                step={1}
                                value={endWeek}
                                disabled={!selectedClassId}
                                onChange={(event) => {
                                    setEndWeek(event.target.value);
                                    setTemplateError(null);
                                }}
                            />
                        </Field>
                    </div>
                    <div className="flex flex-wrap gap-3">
                        <button
                            type="button"
                            className="btn-secondary"
                            onClick={handleDownloadTemplate}
                            disabled={templateLoading || !selectedClassId}
                        >
                            {templateLoading ? "生成中..." : "下载模板"}
                        </button>
                        <button
                            type="button"
                            className="btn-secondary"
                            onClick={handleHistoryRefresh}
                            disabled={historyLoading || !selectedClassId}
                        >
                            {historyLoading ? "加载记录中..." : "刷新历史记录"}
                        </button>
                        {templateError ? (
                            <span className="text-sm text-red-600">{templateError}</span>
                        ) : null}
                    </div>
                </div>
            ) : (
                <p className="text-sm text-slate-500">请先根据筛选条件查询班级。</p>
            )}

            <section className="rounded-2xl border border-slate-200 p-4">
                <h3 className="text-base font-semibold text-slate-900">课次与模板</h3>
                <p className="text-xs text-slate-500">
                    下载模板后可选择具体课次进行导入，未选择时默认显示全部历史记录。
                </p>
                <div className="mt-4 grid gap-4 md:grid-cols-2">
                    <Field label="课次">
                        <select
                            className="input"
                            value={selectedMeetingId}
                            onChange={(event) => setSelectedMeetingId(event.target.value)}
                            disabled={!template || template.meetings.length === 0}
                        >
                            <option value="">全部课次</option>
                            {template?.meetings.map((meeting) => (
                                <option key={meeting.id} value={meeting.id}>
                                    {meetingLabel(meeting)}
                                </option>
                            ))}
                        </select>
                    </Field>
                    {template ? (
                        <div className="rounded-xl bg-slate-50 p-3 text-sm text-slate-600">
                            <p>模板：{template.worksheetName}</p>
                            <p className="mt-1">学生行数：{template.rowCount}</p>
                        </div>
                    ) : (
                        <div className="rounded-xl bg-slate-50 p-3 text-sm text-slate-500">
                            暂未生成模板
                        </div>
                    )}
                </div>
            </section>

            <section className="rounded-2xl border border-slate-200 p-4">
                <h3 className="text-base font-semibold text-slate-900">导入考勤</h3>
                <form className="mt-4 space-y-4" onSubmit={handleUpload}>
                    <div className="grid gap-4 md:grid-cols-2">
                        <Field label="授课老师 / 记录人">
                            <input
                                className="input"
                                value={recordedBy}
                                onChange={(event) => setRecordedBy(event.target.value)}
                                placeholder="选填"
                            />
                        </Field>
                        <Field label="忽略的学生标识（可多行 / 逗号分隔）">
                            <textarea
                                className="input min-h-[80px]"
                                value={ignoredIdentifiers}
                                onChange={(event) => setIgnoredIdentifiers(event.target.value)}
                                placeholder="示例：初二1班-张三"
                            />
                        </Field>
                    </div>
                    <div className="flex flex-wrap gap-4">
                        <label className="btn-secondary">
                            选择 Excel
                            <input
                                type="file"
                                accept=".xlsx,.xls"
                                className="hidden"
                                onChange={handleFileChange}
                            />
                        </label>
                        {selectedFile ? (
                            <span className="text-sm text-slate-600">已选择：{selectedFile.name}</span>
                        ) : (
                            <span className="text-sm text-slate-400">尚未选择文件</span>
                        )}
                    </div>
                    {uploadError ? (
                        <p className="text-sm text-red-600">{uploadError}</p>
                    ) : null}
                    <button
                        type="submit"
                        className="btn-primary"
                        disabled={uploading || !selectedClassId || !selectedMeetingId || !selectedFile}
                    >
                        {uploading ? "导入中..." : "上传并导入"}
                    </button>
                </form>

                {uploadResult ? (
                    <div className="mt-4 rounded-xl bg-slate-50 p-4">
                        <p className="text-sm text-slate-700">
                            导入批次 {uploadResult.batchId}：新增 {uploadResult.inserted} 条、更新 {uploadResult.updated} 条，跳过 {uploadResult.skipped.length} 条。
                        </p>
                        {uploadResult.skipped.length > 0 ? (
                            <div className="mt-3 overflow-x-auto">
                                <table className="w-full text-left text-sm">
                                    <thead>
                                        <tr className="text-slate-500">
                                            <th className="px-2 py-1">Excel 行</th>
                                            <th className="px-2 py-1">学生</th>
                                            <th className="px-2 py-1">状态</th>
                                            <th className="px-2 py-1">备注</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {uploadResult.skipped.map((row) => (
                                            <tr
                                                key={`${row.sourceRow}-${row.studentIdentifier}`}
                                                className="border-t"
                                            >
                                                <td className="px-2 py-1">{row.sourceRow}</td>
                                                <td className="px-2 py-1">{row.studentIdentifier}</td>
                                                <td className="px-2 py-1">
                                                    {formatAttendanceStatus(row.status)}
                                                </td>
                                                <td className="px-2 py-1 text-slate-500">
                                                    {row.note ?? "-"}
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        ) : null}
                    </div>
                ) : null}
            </section>

            <section className="rounded-2xl border border-slate-200 p-4">
                <h3 className="text-base font-semibold text-slate-900">历史记录</h3>
                {historyError ? (
                    <p className="text-sm text-red-600">{historyError}</p>
                ) : null}
                {historyLoading ? (
                    <p className="text-sm text-slate-500">正在读取考勤记录...</p>
                ) : history.length > 0 ? (
                    <div className="mt-3 overflow-x-auto">
                        <table className="w-full text-left text-sm">
                            <thead>
                                <tr className="text-slate-500">
                                    <th className="px-2 py-1">日期</th>
                                    <th className="px-2 py-1">课次</th>
                                    <th className="px-2 py-1">学生</th>
                                    <th className="px-2 py-1">状态</th>
                                    <th className="px-2 py-1">上课时长</th>
                                    <th className="px-2 py-1">记录人</th>
                                    <th className="px-2 py-1">更新时间</th>
                                </tr>
                            </thead>
                            <tbody>
                                {history.map((record) => (
                                    <tr key={record.id} className="border-t">
                                        <td className="px-2 py-1">{record.meetingDate}</td>
                                        <td className="px-2 py-1">第 {record.sessionNumber} 节</td>
                                        <td className="px-2 py-1">{record.studentIdentifier}</td>
                                        <td className="px-2 py-1">{formatAttendanceStatus(record.status)}</td>
                                        <td className="px-2 py-1">{record.minutesAttended ?? "-"}</td>
                                        <td className="px-2 py-1">{record.recordedBy ?? "-"}</td>
                                        <td className="px-2 py-1">{formatTimestamp(record.recordedAt)}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                ) : (
                    <p className="text-sm text-slate-500">尚未加载记录或该条件下暂无数据。</p>
                )}
            </section>
        </div>
    );
}

function meetingLabel(meeting: AttendanceMeeting) {
    return `${meeting.meetingDate} / 第 ${meeting.sessionNumber} 节`;
}

function parseIdentifierInput(value: string): string[] {
    if (!value.trim()) {
        return [];
    }
    return value
        .split(/[\n,;，；]/)
        .map((item) => item.trim())
        .filter((item) => item.length > 0);
}

function formatTimestamp(value: string) {
    const date = new Date(value);
    if (!Number.isFinite(date.getTime())) {
        return value;
    }
    return date.toLocaleString("zh-CN", { hour12: false });
}

function dedupeCampuses(rows: EnrollmentSummaryRow[]): Option[] {
    const ordered = new Map<string, string>();
    rows.forEach((row) => {
        if (!ordered.has(row.campusId)) {
            ordered.set(row.campusId, row.campusName);
        }
    });
    return Array.from(ordered.entries()).map(([value, label]) => ({ value, label }));
}

function dedupeClubs(rows: EnrollmentSummaryRow[], campusId: string): Option[] {
    const ordered = new Map<string, string>();
    rows.forEach((row) => {
        if (row.campusId !== campusId) {
            return;
        }
        if (!ordered.has(row.clubId)) {
            ordered.set(row.clubId, row.clubName);
        }
    });
    return Array.from(ordered.entries()).map(([value, label]) => ({ value, label }));
}

function collectWeekdays(
    rows: EnrollmentSummaryRow[],
    campusId: string,
    clubId: string,
): Option[] {
    const ordered = new Set<string>();
    rows.forEach((row) => {
        if (row.campusId === campusId && row.clubId === clubId) {
            ordered.add(String(row.requestedWeekday));
        }
    });
    return Array.from(ordered).map((value) => ({ value, label: value }));
}

interface AttendanceTemplateState {
    meetings: AttendanceMeeting[];
    worksheetName: string;
    rowCount: number;
    fileName: string;
    fileBase64: string;
    mimeType: string;
}

interface Option {
    value: string;
    label: string;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
    return (
        <label className="flex flex-col gap-1 text-sm text-slate-700">
            <span className="text-xs font-medium uppercase tracking-wide text-slate-500">
                {label}
            </span>
            {children}
        </label>
    );
}
