"use client";

import { useState } from "react";

import type { StudentImportSummary } from "@/lib/types";
import {
    StudentRosterServiceError,
    importStudentExcel,
} from "@/services/studentRosterService";

import { ExcelDropzone } from "./ExcelDropzone";

interface StudentImportPanelProps {
    onCompleted?: () => void;
}

export function StudentImportPanel({ onCompleted }: StudentImportPanelProps) {
    const [summary, setSummary] = useState<StudentImportSummary | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [uploading, setUploading] = useState(false);

    const handleFileSelected = async (file: File) => {
        setError(null);
        setUploading(true);
        try {
            const result = await importStudentExcel(file);
            setSummary(result);
            onCompleted?.();
        } catch (err) {
            if (err instanceof StudentRosterServiceError) {
                setError(err.message);
            } else if (err instanceof Error) {
                setError(err.message);
            } else {
                setError("未知错误，请稍后再试");
            }
        } finally {
            setUploading(false);
        }
    };

    return (
        <div className="space-y-4">
            <ExcelDropzone onFileSelected={handleFileSelected} disabled={uploading} />
            {uploading && <p className="text-sm text-slate-500">正在上传并解析 Excel ...</p>}
            {error && <p className="text-sm text-red-600">错误：{error}</p>}
            {summary && (
                <div className="space-y-2">
                    <p className="text-sm text-slate-600">
                        导入完成：成功 {summary.successRows} 行 / 跳过 {summary.skippedRows} 行 / 共{" "}
                        {summary.totalRows} 行。
                    </p>
                    {summary.errors.length > 0 ? (
                        <div className="rounded-xl border border-amber-200 bg-amber-50 p-4">
                            <p className="text-sm font-medium text-amber-800">
                                共有 {summary.errors.length} 条错误需要处理：
                            </p>
                            <ul className="mt-2 space-y-1 text-sm text-amber-900">
                                {summary.errors.map((item) => (
                                    <li key={`error-${item.row}`}>
                                        第 {item.row} 行：{item.message}
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ) : (
                        <p className="text-sm text-emerald-600">未发现导入错误。</p>
                    )}
                </div>
            )}
        </div>
    );
}
