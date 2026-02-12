"use client";

import { useState } from "react";

import type {
    CampusOption,
    TermOption,
} from "@/components/students/StudentRosterWorkspace";
import type { TeacherChildImportSummary } from "@/lib/types";
import {
    StudentRosterServiceError,
    TeacherChildImportMode,
    TeacherChildImportConfig,
    importTeacherChildrenExcel,
} from "@/services/studentRosterService";

import { ExcelDropzone } from "./ExcelDropzone";

interface TeacherChildImportPanelProps {
    terms: TermOption[];
    campuses: CampusOption[];
    defaultTermId?: string;
    onCompleted?: () => void;
}

const DEFAULT_CLASS_COLUMN = "B";
const DEFAULT_STUDENT_COLUMN = "C";
const DEFAULT_COMBINED_COLUMN = "E";

export function TeacherChildImportPanel({
    terms,
    campuses,
    defaultTermId,
    onCompleted,
}: TeacherChildImportPanelProps) {
    const [selectedTermId, setSelectedTermId] = useState(
        defaultTermId ?? terms[0]?.id ?? "",
    );
    const [selectedCampusId, setSelectedCampusId] = useState(campuses[0]?.id ?? "");
    const [mode, setMode] = useState<TeacherChildImportMode>("SPLIT");
    const [classColumn, setClassColumn] = useState(DEFAULT_CLASS_COLUMN);
    const [studentColumn, setStudentColumn] = useState(DEFAULT_STUDENT_COLUMN);
    const [combinedColumn, setCombinedColumn] = useState(DEFAULT_COMBINED_COLUMN);
    const [summary, setSummary] = useState<TeacherChildImportSummary | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [uploading, setUploading] = useState(false);

    const handleFileSelected = async (file: File) => {
        if (!selectedTermId || !selectedCampusId) {
            setError("请先选择学期与校区再上传 Excel。");
            return;
        }
        const columnResult = buildColumnConfig(
            mode,
            classColumn,
            studentColumn,
            combinedColumn,
        );
        if (columnResult.error) {
            setError(columnResult.error);
            return;
        }
        setUploading(true);
        setError(null);
        try {
            const result = await importTeacherChildrenExcel({
                termId: selectedTermId,
                campusId: selectedCampusId,
                file,
                config: columnResult.config,
            });
            setSummary(result);
            onCompleted?.();
        } catch (err) {
            if (err instanceof StudentRosterServiceError) {
                setError(err.message);
            } else if (err instanceof Error) {
                setError(err.message);
            } else {
                setError("未知错误，请稍后重试。");
            }
        } finally {
            setUploading(false);
        }
    };

    const handleResetColumns = () => {
        setClassColumn(DEFAULT_CLASS_COLUMN);
        setStudentColumn(DEFAULT_STUDENT_COLUMN);
        setCombinedColumn(DEFAULT_COMBINED_COLUMN);
    };

    return (
        <div className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
                <label className="text-sm font-medium text-slate-700">
                    <span className="mb-1 block text-xs text-slate-500">学期</span>
                    <select
                        value={selectedTermId}
                        onChange={(event) => setSelectedTermId(event.target.value)}
                        className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    >
                        <option value="">请选择学期</option>
                        {terms.map((term) => (
                            <option key={term.id} value={term.id}>
                                {term.name}
                                {term.isActive ? "（当前）" : ""}
                            </option>
                        ))}
                    </select>
                </label>
                <label className="text-sm font-medium text-slate-700">
                    <span className="mb-1 block text-xs text-slate-500">校区</span>
                    <select
                        value={selectedCampusId}
                        onChange={(event) => setSelectedCampusId(event.target.value)}
                        className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    >
                        <option value="">请选择校区</option>
                        {campuses.map((campus) => (
                            <option key={campus.id} value={campus.id}>
                                {campus.name}
                                {campus.shortName ? `（${campus.shortName}）` : ""}
                            </option>
                        ))}
                    </select>
                </label>
            </div>
            <section className="rounded-2xl border border-slate-200 bg-white p-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                        <p className="text-sm font-semibold text-slate-900">Excel 列配置</p>
                        <p className="text-xs text-slate-500">
                            默认读取 B 列班级、C 列学生姓名。从第 2 行开始，空行自动跳过。
                        </p>
                    </div>
                    <div className="flex flex-wrap items-center gap-4 text-sm text-slate-700">
                        <label className="flex items-center gap-2">
                            <input
                                type="radio"
                                name="teacher-child-mode"
                                value="SPLIT"
                                checked={mode === "SPLIT"}
                                onChange={() => setMode("SPLIT")}
                                className="h-4 w-4 text-indigo-600 focus:ring-indigo-500"
                            />
                            班级列 + 姓名列
                        </label>
                        <label className="flex items-center gap-2">
                            <input
                                type="radio"
                                name="teacher-child-mode"
                                value="COMBINED"
                                checked={mode === "COMBINED"}
                                onChange={() => setMode("COMBINED")}
                                className="h-4 w-4 text-indigo-600 focus:ring-indigo-500"
                            />
                            同列（班级 + 姓名）
                        </label>
                    </div>
                </div>
                {mode === "SPLIT" ? (
                    <div className="mt-4 grid gap-4 sm:grid-cols-2">
                        <ColumnField
                            label="班级列"
                            placeholder="例如 B 或 2"
                            value={classColumn}
                            onChange={setClassColumn}
                        />
                        <ColumnField
                            label="姓名列"
                            placeholder="例如 C 或 3"
                            value={studentColumn}
                            onChange={setStudentColumn}
                        />
                    </div>
                ) : (
                    <div className="mt-4 grid gap-4 sm:grid-cols-2">
                        <ColumnField
                            label="班级 + 姓名所在列"
                            placeholder="例如 E 或 5"
                            value={combinedColumn}
                            onChange={setCombinedColumn}
                        />
                        <p className="text-xs text-slate-500">
                            支持“班级 姓名”、“班级-姓名”或“[学号]班级姓名”等组合格式。
                        </p>
                    </div>
                )}
                <div className="mt-3 flex items-center justify-between text-xs text-slate-500">
                    <span>支持列字母（A、B...）或列序号（1 表示 A 列）。</span>
                    <button
                        type="button"
                        onClick={handleResetColumns}
                        className="text-xs font-medium text-indigo-600 transition hover:text-indigo-500"
                    >
                        恢复默认
                    </button>
                </div>
            </section>
            <ExcelDropzone onFileSelected={handleFileSelected} disabled={uploading} />
            {uploading && <p className="text-sm text-slate-500">正在上传并解析 Excel ...</p>}
            {error && <p className="text-sm text-red-600">错误：{error}</p>}
            {summary && (
                <SummaryCard summary={summary} />
            )}
        </div>
    );
}

function ColumnField({
    label,
    value,
    onChange,
    placeholder,
}: {
    label: string;
    value: string;
    placeholder: string;
    onChange: (value: string) => void;
}) {
    return (
        <label className="space-y-1 text-sm font-medium text-slate-700">
            <span>{label}</span>
            <input
                type="text"
                maxLength={4}
                value={value}
                placeholder={placeholder}
                onChange={(event) => onChange(sanitizeColumnInput(event.target.value))}
                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            />
        </label>
    );
}

function SummaryCard({ summary }: { summary: TeacherChildImportSummary }) {
    return (
        <div className="space-y-2 rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-700">
            <p>
                共读取 {summary.totalRows} 行；匹配 {summary.matchedStudents} 人，其中{" "}
                <span className="font-semibold text-emerald-600">{summary.updatedStudents}</span> 人已成功标记为教师子女，
                <span className="font-semibold text-slate-900">{summary.alreadyMarked}</span> 人原本已标记。
            </p>
            <p>
                跳过空行 {summary.skippedRows} 行，重复 {summary.duplicateRows} 行，错误{" "}
                {summary.errors.length} 行。
            </p>
            {summary.errors.length > 0 && (
                <div className="rounded-xl border border-amber-200 bg-amber-50 p-3 text-amber-900">
                    <p className="text-sm font-semibold">错误明细：</p>
                    <ul className="mt-1 space-y-1 text-xs">
                        {summary.errors.map((item) => (
                            <li key={`teacher-child-error-${item.row}`}>
                                第 {item.row} 行：{item.message}
                            </li>
                        ))}
                    </ul>
                </div>
            )}
        </div>
    );
}

function buildColumnConfig(
    mode: TeacherChildImportMode,
    classColumn: string,
    studentColumn: string,
    combinedColumn: string,
): { config?: TeacherChildImportConfig; error?: string } {
    if (mode === "COMBINED") {
        const normalized = sanitizeColumnInput(combinedColumn);
        if (!isValidColumn(normalized)) {
            return { error: "请填写有效的“班级 + 姓名”列（示例：E 或 5）。" };
        }
        return {
            config: {
                mode,
                combinedColumn: normalized,
            },
        };
    }

    const normalizedClass = sanitizeColumnInput(classColumn);
    const normalizedStudent = sanitizeColumnInput(studentColumn);
    if (!isValidColumn(normalizedClass)) {
        return { error: "请填写有效的班级列（示例：B 或 2）。" };
    }
    if (!isValidColumn(normalizedStudent)) {
        return { error: "请填写有效的姓名列（示例：C 或 3）。" };
    }
    return {
        config: {
            mode,
            classColumn: normalizedClass,
            studentColumn: normalizedStudent,
        },
    };
}

function sanitizeColumnInput(value: string): string {
    return value.replace(/\s+/g, "").toUpperCase();
}

function isValidColumn(value: string): boolean {
    if (!value) {
        return false;
    }
    return /^[A-Z]+$/.test(value) || /^\d+$/.test(value);
}
