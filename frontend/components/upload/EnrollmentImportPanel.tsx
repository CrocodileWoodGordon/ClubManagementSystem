"use client";

import { useMemo, useState } from "react";

import type { EnrollmentImportOutcome } from "@/lib/types";
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

    const summary = useMemo(() => summarize(outcomes), [outcomes]);

    const handleFileSelected = async (file: File) => {
        setError(null);
        setIsUploading(true);
        try {
            const result = await importEnrollmentExcel({ file });
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
