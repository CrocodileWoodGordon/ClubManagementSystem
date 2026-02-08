"use client";

import { useMemo, useState } from "react";

import type { ImportPlaceholderConfig, ImportPlaceholderType } from "@/lib/types";
import { updateImportPlaceholders } from "@/services/importPlaceholderService";

interface Props {
    initialConfigs: ImportPlaceholderConfig[];
}

type PlaceholderState = {
    placeholders: string[];
    dirty: boolean;
    error: string | null;
    success: string | null;
};

const SUPPORTED_TYPES: ImportPlaceholderType[] = ["ENROLLMENTS", "STUDENTS"];

const TYPE_LABELS: Record<ImportPlaceholderType, { title: string; description: string }> = {
    ENROLLMENTS: {
        title: "问卷星报名导入",
        description: "这些字符串会被视为“空报名”，导入时跳过不生成社团/报名记录。",
    },
    STUDENTS: {
        title: "学生名单导入",
        description: "匹配“校区/班级/姓名”文件时自动忽略以下占位内容。",
    },
};

export function PlaceholderSettingsPanel({ initialConfigs }: Props) {
    const initialState = useMemo(() => {
        const base = SUPPORTED_TYPES.reduce<Record<ImportPlaceholderType, PlaceholderState>>(
            (acc, type) => {
                acc[type] = { placeholders: [], dirty: false, error: null, success: null };
                return acc;
            },
            {} as Record<ImportPlaceholderType, PlaceholderState>,
        );
        for (const config of initialConfigs) {
            base[config.importType] = {
                placeholders: [...config.placeholders],
                dirty: false,
                error: null,
                success: null,
            };
        }
        return base;
    }, [initialConfigs]);

    const [state, setState] = useState(initialState);
    const [newInputs, setNewInputs] = useState<Record<ImportPlaceholderType, string>>(
        SUPPORTED_TYPES.reduce((acc, type) => {
            acc[type] = "";
            return acc;
        }, {} as Record<ImportPlaceholderType, string>),
    );
    const [savingType, setSavingType] = useState<ImportPlaceholderType | null>(null);

    const handleAdd = (importType: ImportPlaceholderType) => {
        const value = newInputs[importType].trim();
        if (!value) {
            setState((prev) => ({
                ...prev,
                [importType]: { ...prev[importType], error: "请输入占位字符串", success: null },
            }));
            return;
        }
        const normalized = value.toLowerCase();
        const exists = state[importType].placeholders.some(
            (item) => item.toLowerCase() === normalized,
        );
        if (exists) {
            setState((prev) => ({
                ...prev,
                [importType]: {
                    ...prev[importType],
                    error: "该占位文本已存在",
                    success: null,
                },
            }));
            return;
        }
        setState((prev) => ({
            ...prev,
            [importType]: {
                placeholders: [...prev[importType].placeholders, value],
                dirty: true,
                error: null,
                success: null,
            },
        }));
        setNewInputs((prev) => ({ ...prev, [importType]: "" }));
    };

    const handleRemove = (importType: ImportPlaceholderType, value: string) => {
        setState((prev) => ({
            ...prev,
            [importType]: {
                placeholders: prev[importType].placeholders.filter((item) => item !== value),
                dirty: true,
                error: null,
                success: null,
            },
        }));
    };

    const handleSave = (importType: ImportPlaceholderType) => {
        const placeholders = state[importType].placeholders;
        setSavingType(importType);
        updateImportPlaceholders(importType, placeholders)
            .then((updated) => {
                setState((prev) => ({
                    ...prev,
                    [importType]: {
                        placeholders: [...updated.placeholders],
                        dirty: false,
                        error: null,
                        success: "已保存",
                    },
                }));
            })
            .catch((error) => {
                setState((prev) => ({
                    ...prev,
                    [importType]: {
                        ...prev[importType],
                        error: error instanceof Error ? error.message : String(error),
                        success: null,
                    },
                }));
            })
            .finally(() => {
                setSavingType((current) => (current === importType ? null : current));
            });
    };

    return (
        <div className="space-y-6">
            {SUPPORTED_TYPES.map((importType) => {
                const info = TYPE_LABELS[importType];
                const placeholders = state[importType].placeholders;
                const isSaving = savingType === importType;
                return (
                    <section key={importType} className="rounded-xl border border-slate-200 p-4">
                        <div className="space-y-1">
                            <h3 className="text-lg font-semibold text-slate-900">{info.title}</h3>
                            <p className="text-sm text-slate-600">{info.description}</p>
                        </div>
                        <div className="mt-4 space-y-4">
                            <div className="flex flex-wrap gap-2">
                                {placeholders.length === 0 ? (
                                    <span className="text-sm text-slate-500">暂无占位文本。</span>
                                ) : (
                                    placeholders.map((value) => (
                                        <span
                                            key={value}
                                            className="inline-flex items-center gap-2 rounded-full border border-slate-300 px-3 py-1 text-sm text-slate-700"
                                        >
                                            {value}
                                            <button
                                                type="button"
                                                className="text-slate-400 transition hover:text-red-500"
                                                onClick={() => handleRemove(importType, value)}
                                                aria-label={`移除 ${value}`}
                                            >
                                                ×
                                            </button>
                                        </span>
                                    ))
                                )}
                            </div>
                            <div className="flex flex-wrap gap-2">
                                <input
                                    type="text"
                                    value={newInputs[importType]}
                                    onChange={(event) =>
                                        setNewInputs((prev) => ({
                                            ...prev,
                                            [importType]: event.target.value,
                                        }))
                                    }
                                    placeholder="输入占位字符串"
                                    className="flex-1 min-w-[200px] rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                                />
                                <button
                                    type="button"
                                    className="rounded-md border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 transition hover:bg-slate-50"
                                    onClick={() => handleAdd(importType)}
                                >
                                    新增
                                </button>
                            </div>
                            <div className="flex flex-wrap items-center gap-3 text-sm">
                                <button
                                    type="button"
                                    onClick={() => handleSave(importType)}
                                    disabled={!state[importType].dirty || isSaving}
                                    className="rounded-md bg-indigo-600 px-4 py-2 font-medium text-white transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-300"
                                >
                                    {isSaving ? "保存中..." : "保存设置"}
                                </button>
                                {state[importType].error ? (
                                    <span className="text-red-600">{state[importType].error}</span>
                                ) : state[importType].success ? (
                                    <span className="text-green-600">{state[importType].success}</span>
                                ) : (
                                    <span className="text-slate-500">保存后即时生效。</span>
                                )}
                            </div>
                        </div>
                    </section>
                );
            })}
        </div>
    );
}
