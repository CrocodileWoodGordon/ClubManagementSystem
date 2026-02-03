"use client";

import { useState } from "react";

interface BulkAssignmentFormProps {
    onSubmit: (payload: { studentIds: string[]; batchNumber: string }) => Promise<void>;
}

export function BulkAssignmentForm({ onSubmit }: BulkAssignmentFormProps) {
    const [batchNumber, setBatchNumber] = useState("1");
    const [selectedCount, setSelectedCount] = useState(0);

    const handleSubmit = async () => {
        await onSubmit({ studentIds: [], batchNumber });
    };

    return (
        <div className="rounded-2xl border border-dashed border-slate-300 bg-white p-4 space-y-2">
            <p className="text-sm text-slate-500">批量操作</p>
            <div className="flex items-center gap-3">
                <label className="text-sm text-slate-600">班级编号</label>
                <input
                    value={batchNumber}
                    onChange={(event) => setBatchNumber(event.target.value)}
                    className="rounded-lg border border-slate-200 px-2 py-1 text-sm"
                />
                <button
                    onClick={handleSubmit}
                    className="rounded-lg bg-slate-900 px-3 py-1.5 text-sm font-medium text-white"
                >
                    应用到 {selectedCount} 人
                </button>
            </div>
        </div>
    );
}
