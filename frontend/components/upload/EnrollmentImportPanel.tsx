"use client";

import { useMemo, useState } from "react";

import type {
    ColumnReference,
    EnrollmentImportConfig,
    EnrollmentImportOutcome,
} from "@/lib/types";
import { formatWeekday } from "@/lib/utils";
import {
    EnrollmentServiceError,
    importEnrollmentExcel,
} from "@/services/enrollmentService";

import { ExcelDropzone } from "./ExcelDropzone";

interface EnrollmentImportPanelProps {
    onCompleted?: () => void;
}

const STATUS_LABELS: Record<string, string> = {
    PENDING: "待处理",
    CREATED: "已创建",
    SKIPPED: "已跳过",
    FAILED: "失败",
};

export function EnrollmentImportPanel({ onCompleted }: EnrollmentImportPanelProps) {
    const [isUploading, setIsUploading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [outcomes, setOutcomes] = useState<EnrollmentImportOutcome[]>([]);
    const [useCustomColumns, setUseCustomColumns] = useState(false);
    const [columnForm, setColumnForm] = useState<ColumnFormState>(() => createDefaultColumnForm());

    const summary = useMemo(() => summarize(outcomes), [outcomes]);

    const handleFileSelected = async (file: File) => {
        setError(null);
        const columnConfigResult = buildColumnConfig(useCustomColumns, columnForm);
        if (columnConfigResult.error) {
            setError(columnConfigResult.error);
            return;
        }
        setIsUploading(true);
        try {
            const result = await importEnrollmentExcel({
                file,
                config: columnConfigResult.config,
            });
            setOutcomes(result);
            if (onCompleted) {
                onCompleted();
            }
        } catch (err) {
            if (err instanceof EnrollmentServiceError) {
                setError(err.message);
            } else if (err instanceof Error) {
                setError(err.message);
            } else {
                setError("未知错误，请稍后重试");
            }
        } finally {
            setIsUploading(false);
        }
    };

    return (
        <div className="space-y-4">
            <ColumnConfigPanel
                form={columnForm}
                useCustomColumns={useCustomColumns}
                onToggle={(next) => {
                    setUseCustomColumns(next);
                    setError(null);
                }}
                onChangeStudentColumn={(value) =>
                    setColumnForm((prev) => ({ ...prev, studentColumn: value }))
                }
                onChangeWeekdayColumn={(day, value) =>
                    setColumnForm((prev) => ({
                        ...prev,
                        weekdayColumns: { ...prev.weekdayColumns, [day]: value },
                    }))
                }
                onReset={() => setColumnForm(createDefaultColumnForm())}
            />
            <ExcelDropzone onFileSelected={handleFileSelected} disabled={isUploading} />
            {isUploading && <p className="text-sm text-slate-500">正在上传并解析 Excel ...</p>}
            {error && <p className="text-sm text-red-600">错误：{error}</p>}
            {outcomes.length > 0 && (
                <div className="space-y-2">
                    <p className="text-sm text-slate-600">
                        导入汇总：成功 {summary.created} 行 / 跳过 {summary.skipped} 行 / 失败{" "}
                        {summary.failed} 行（共 {outcomes.length} 行）
                    </p>
                    <OutcomeTable outcomes={outcomes} />
                </div>
            )}
        </div>
    );
}

function OutcomeTable({ outcomes }: { outcomes: EnrollmentImportOutcome[] }) {
    return (
        <div className="max-h-72 overflow-y-auto rounded-xl border border-slate-200">
            <table className="min-w-full text-sm">
                <thead className="bg-slate-50 text-left text-slate-500">
                    <tr>
                        <th className="px-3 py-2">Excel 行</th>
                        <th className="px-3 py-2">学生</th>
                        <th className="px-3 py-2">星期</th>
                        <th className="px-3 py-2">社团/班级</th>
                        <th className="px-3 py-2">状态</th>
                        <th className="px-3 py-2">描述</th>
                    </tr>
                </thead>
                <tbody>
                    {outcomes.map((outcome) => (
                        <tr key={outcome.id} className="border-t">
                            <td className="px-3 py-2 text-slate-600">{outcome.sourceRow}</td>
                            <td className="px-3 py-2 text-slate-900">
                                {outcome.draft?.studentFullName ?? "--"}
                            </td>
                            <td className="px-3 py-2 text-slate-600">
                                {outcome.draft ? formatWeekday(outcome.draft.requestedWeekday) : "--"}
                            </td>
                            <td className="px-3 py-2 text-slate-600">
                                {outcome.draft?.clubLookupValue ?? outcome.draft?.homeroomDisplayName ?? "--"}
                            </td>
                            <td className="px-3 py-2 font-medium text-slate-900">
                                {STATUS_LABELS[outcome.status] ?? outcome.status}
                            </td>
                            <td className="px-3 py-2 text-slate-600">
                                {outcome.message ?? "—"}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

function summarize(outcomes: EnrollmentImportOutcome[]) {
    return outcomes.reduce(
        (acc, item) => {
            switch (item.status) {
                case "CREATED":
                    acc.created += 1;
                    break;
                case "SKIPPED":
                    acc.skipped += 1;
                    break;
                case "FAILED":
                    acc.failed += 1;
                    break;
                default:
                    acc.pending += 1;
            }
            return acc;
        },
        { created: 0, skipped: 0, failed: 0, pending: 0 },
    );
}

type ColumnFormState = {
    studentColumn: string;
    weekdayColumns: Record<number, string>;
};

const WEEKDAY_FIELDS: { day: number; label: string; defaultColumn: string }[] = [
    { day: 1, label: "周一社团列", defaultColumn: "H" },
    { day: 2, label: "周二社团列", defaultColumn: "I" },
    { day: 3, label: "周三社团列", defaultColumn: "J" },
    { day: 4, label: "周四社团列", defaultColumn: "K" },
    { day: 5, label: "周五社团列", defaultColumn: "L" },
];

const DEFAULT_STUDENT_COLUMN = "E";

function createDefaultColumnForm(): ColumnFormState {
    const weekdayColumns: Record<number, string> = {};
    WEEKDAY_FIELDS.forEach((field) => {
        weekdayColumns[field.day] = field.defaultColumn;
    });
    return {
        studentColumn: DEFAULT_STUDENT_COLUMN,
        weekdayColumns,
    };
}

function ColumnConfigPanel({
    form,
    useCustomColumns,
    onToggle,
    onChangeStudentColumn,
    onChangeWeekdayColumn,
    onReset,
}: {
    form: ColumnFormState;
    useCustomColumns: boolean;
    onToggle: (value: boolean) => void;
    onChangeStudentColumn: (value: string) => void;
    onChangeWeekdayColumn: (day: number, value: string) => void;
    onReset: () => void;
}) {
    return (
        <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <p className="text-sm font-semibold text-slate-900">Excel 列映射</p>
                    <p className="text-xs text-slate-500">
                        默认遵循问卷星模板（E 列班级+姓名，H~L 列分别代表周一~周五社团）。
                    </p>
                </div>
                <label className="flex items-center gap-2 text-sm font-medium text-slate-700">
                    <input
                        type="checkbox"
                        className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                        checked={useCustomColumns}
                        onChange={(event) => onToggle(event.target.checked)}
                    />
                    启用自定义
                </label>
            </div>
            {useCustomColumns && (
                <div className="mt-4 space-y-4">
                    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">
                                班级 + 姓名所在列
                            </label>
                            <input
                                type="text"
                                value={form.studentColumn}
                                maxLength={4}
                                onChange={(event) =>
                                    onChangeStudentColumn(sanitizeColumnInput(event.target.value))
                                }
                                placeholder="例如 E 或 5"
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                        <div className="sm:col-span-1 lg:col-span-2">
                            <p className="text-xs text-slate-500">
                                支持 Excel 列字母（A、B...AA）或列序号（1 表示 A 列）。
                            </p>
                        </div>
                    </div>
                    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
                        {WEEKDAY_FIELDS.map((field) => (
                            <div key={field.day} className="space-y-1">
                                <label className="text-sm font-medium text-slate-700">
                                    {field.label}
                                </label>
                                <input
                                    type="text"
                                    value={form.weekdayColumns[field.day]}
                                    maxLength={4}
                                    onChange={(event) =>
                                        onChangeWeekdayColumn(
                                            field.day,
                                            sanitizeColumnInput(event.target.value),
                                        )
                                    }
                                    placeholder={field.defaultColumn}
                                    className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                                />
                            </div>
                        ))}
                    </div>
                    <div className="flex flex-wrap items-center gap-3 text-xs text-slate-500">
                        <span>如遇周末社团，可自行填写相应列位置。</span>
                        <button
                            type="button"
                            onClick={onReset}
                            className="text-xs font-medium text-indigo-600 transition hover:text-indigo-500"
                        >
                            恢复默认映射
                        </button>
                    </div>
                </div>
            )}
        </section>
    );
}

function buildColumnConfig(
    enabled: boolean,
    form: ColumnFormState,
): { config?: EnrollmentImportConfig; error?: string } {
    if (!enabled) {
        return { config: undefined };
    }

    const studentColumn = normalizeColumnReference(form.studentColumn);
    if (!studentColumn) {
        return { error: "请填写有效的“班级 + 姓名”列（示例：E 或 5）。" };
    }

    const weekdayColumns: Record<number, ColumnReference> = {};
    for (const field of WEEKDAY_FIELDS) {
        const column = normalizeColumnReference(form.weekdayColumns[field.day]);
        if (!column) {
            return {
                error: `${field.label} 需要填写有效列（示例：${field.defaultColumn} 或 8）。`,
            };
        }
        weekdayColumns[field.day] = column;
    }

    return {
        config: {
            studentColumn,
            weekdayColumns,
        },
    };
}

function sanitizeColumnInput(value: string): string {
    return value.replace(/\s+/g, "").toUpperCase();
}

function normalizeColumnReference(value: string): ColumnReference | null {
    const trimmed = value.trim();
    if (!trimmed) {
        return null;
    }
    if (/^\d+$/.test(trimmed)) {
        const numeric = Number(trimmed);
        return numeric > 0 ? numeric : null;
    }
    if (/^[A-Za-z]+$/.test(trimmed)) {
        return trimmed.toUpperCase();
    }
    return null;
}
