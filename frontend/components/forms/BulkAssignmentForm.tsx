"use client";

import { useMemo, useState } from "react";

import type { ClassInstance } from "@/lib/types";

interface BulkAssignmentFormProps {
    classes: ClassInstance[];
    selectedCount: number;
    disabled?: boolean;
    onApply: (classId: string | null) => Promise<void>;
}

export function BulkAssignmentForm({
    classes,
    selectedCount,
    onApply,
    disabled = false,
}: BulkAssignmentFormProps) {
    const [targetClassId, setTargetClassId] = useState<string>("");
    const [submitting, setSubmitting] = useState(false);

    const selectOptions = useMemo(
        () => [
            { value: "", label: "待定班（清除分班）" },
            ...classes.map((cls) => ({
                value: cls.id,
                label: `${cls.classCode}｜${cls.startTime}-${cls.endTime}`,
            })),
        ],
        [classes],
    );

    const handleSubmit = async () => {
        if (selectedCount === 0 || disabled) {
            return;
        }
        setSubmitting(true);
        try {
            await onApply(targetClassId || null);
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="rounded-2xl border border-dashed border-slate-300 bg-white p-4 space-y-3">
            <div className="flex flex-col gap-1">
                <p className="text-sm font-medium text-slate-900">批量设置分班</p>
                <p className="text-xs text-slate-500">选择目标班级后，可一次性同步到所有已勾选学生。</p>
            </div>
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                <label className="text-sm text-slate-600">目标班级</label>
                <select
                    value={targetClassId}
                    onChange={(event) => setTargetClassId(event.target.value)}
                    className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none sm:w-auto"
                >
                    {selectOptions.map((option) => (
                        <option key={option.value} value={option.value}>
                            {option.label}
                        </option>
                    ))}
                </select>
                <button
                    type="button"
                    onClick={handleSubmit}
                    disabled={selectedCount === 0 || disabled || submitting}
                    className="inline-flex w-full items-center justify-center rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-300 sm:w-auto"
                >
                    {submitting ? "应用中..." : `应用到 ${selectedCount} 人`}
                </button>
            </div>
        </div>
    );
}
