"use client";

import { useMemo, useState } from "react";

import type { Term } from "@/lib/types";
import { activateTerm, createTerm, deleteTerm, updateTerm } from "@/services/termService";

interface Props {
    initialTerms: Term[];
}

type FormState = {
    code: string;
    name: string;
    startDate: string;
    endDate: string;
    enrollmentStart: string;
    enrollmentEnd: string;
    isActive: boolean;
};

type Feedback = {
    type: "success" | "error";
    text: string;
} | null;

const EMPTY_FORM: FormState = {
    code: "",
    name: "",
    startDate: "",
    endDate: "",
    enrollmentStart: "",
    enrollmentEnd: "",
    isActive: false,
};

function sortTerms(items: Term[]) {
    return [...items].sort((a, b) => b.startDate.localeCompare(a.startDate));
}

export function TermSettingsPanel({ initialTerms }: Props) {
    const [terms, setTerms] = useState<Term[]>(() => sortTerms(initialTerms));
    const [formState, setFormState] = useState<FormState>(EMPTY_FORM);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [pendingAction, setPendingAction] = useState<string | null>(null);
    const [feedback, setFeedback] = useState<Feedback>(null);

    const isEditing = editingId !== null;
    const currentActionLabel = isEditing ? "保存修改" : "创建学期";

    const activeTermId = useMemo(() => {
        const active = terms.find((term) => term.isActive);
        return active?.id ?? null;
    }, [terms]);

    const handleEdit = (term: Term) => {
        setEditingId(term.id);
        setFormState({
            code: term.code,
            name: term.name,
            startDate: term.startDate,
            endDate: term.endDate,
            enrollmentStart: term.enrollmentStart,
            enrollmentEnd: term.enrollmentEnd,
            isActive: term.isActive,
        });
        setFeedback(null);
    };

    const resetForm = () => {
        setEditingId(null);
        setFormState(EMPTY_FORM);
    };

    const validateForm = () => {
        if (!formState.code.trim()) {
            return "请填写学期编号";
        }
        if (!formState.name.trim()) {
            return "请填写学期名称";
        }
        if (!formState.startDate || !formState.endDate) {
            return "请填写学期起止日期";
        }
        if (!formState.enrollmentStart || !formState.enrollmentEnd) {
            return "请填写报名开放区间";
        }
        return null;
    };

    const handleSubmit: React.FormEventHandler<HTMLFormElement> = async (event) => {
        event.preventDefault();
        setFeedback(null);

        const validationError = validateForm();
        if (validationError) {
            setFeedback({ type: "error", text: validationError });
            return;
        }

        setPendingAction("save");
        try {
            if (isEditing && editingId) {
                const updated = await updateTerm(editingId, formState);
                setTerms((prev) =>
                    sortTerms(prev.map((term) => (term.id === updated.id ? updated : term))),
                );
                setFeedback({ type: "success", text: "学期信息已更新" });
            } else {
                const created = await createTerm(formState);
                setTerms((prev) => sortTerms([...prev, created]));
                setFeedback({ type: "success", text: "学期已创建" });
            }
            resetForm();
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setFeedback({ type: "error", text: message });
        } finally {
            setPendingAction((value) => (value === "save" ? null : value));
        }
    };

    const handleActivate = async (termId: string) => {
        setPendingAction(`activate-${termId}`);
        setFeedback(null);
        try {
            const updated = await activateTerm(termId);
            setTerms((prev) =>
                sortTerms(
                    prev.map((term) =>
                        term.id === updated.id ? updated : { ...term, isActive: false },
                    ),
                ),
            );
            setFeedback({ type: "success", text: `已切换至 ${updated.name}` });
            if (editingId === termId) {
                setFormState((prev) => ({ ...prev, isActive: true }));
            } else if (editingId && editingId !== termId && activeTermId === editingId) {
                setFormState((prev) => ({ ...prev, isActive: false }));
            }
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setFeedback({ type: "error", text: message });
        } finally {
            setPendingAction((value) => (value === `activate-${termId}` ? null : value));
        }
    };

    const handleDelete = async (term: Term) => {
        const confirmed = window.confirm(`确认删除学期「${term.name}」？该操作无法撤销。`);
        if (!confirmed) return;

        setPendingAction(`delete-${term.id}`);
        setFeedback(null);
        try {
            await deleteTerm(term.id);
            setTerms((prev) => sortTerms(prev.filter((item) => item.id !== term.id)));
            setFeedback({ type: "success", text: "学期已删除" });
            if (editingId === term.id) {
                resetForm();
            }
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setFeedback({ type: "error", text: message });
        } finally {
            setPendingAction((value) => (value === `delete-${term.id}` ? null : value));
        }
    };

    const handleInputChange = <K extends keyof FormState>(field: K, value: FormState[K]) => {
        setFormState((prev) => ({
            ...prev,
            [field]: value,
        }));
    };

    const renderFeedback = () => {
        if (!feedback) return null;
        const baseClass =
            feedback.type === "success"
                ? "bg-emerald-50 text-emerald-700 border-emerald-200"
                : "bg-rose-50 text-rose-700 border-rose-200";
        return (
            <div className={`rounded-lg border px-3 py-2 text-sm ${baseClass}`}>
                {feedback.text}
            </div>
        );
    };

    return (
        <div className="space-y-8">
            {renderFeedback()}
            <section className="space-y-4">
                <div>
                    <h3 className="text-lg font-semibold text-slate-900">学期列表</h3>
                    <p className="text-sm text-slate-600">
                        查看全部学期，快速切换当前生效学期或更新基础信息。
                    </p>
                </div>
                <div className="overflow-hidden rounded-xl border border-slate-200 bg-white">
                    <table className="min-w-full divide-y divide-slate-200 text-sm">
                        <thead className="bg-slate-50 text-left text-xs font-semibold uppercase tracking-wide text-slate-500">
                            <tr>
                                <th className="px-4 py-3">学期</th>
                                <th className="px-4 py-3">起止日期</th>
                                <th className="px-4 py-3">报名开放</th>
                                <th className="px-4 py-3 text-right">操作</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-slate-100 bg-white">
                            {terms.length === 0 ? (
                                <tr>
                                    <td
                                        colSpan={4}
                                        className="px-4 py-6 text-center text-sm text-slate-500"
                                    >
                                        暂无学期，请先创建。
                                    </td>
                                </tr>
                            ) : (
                                terms.map((term) => {
                                    const isActivating = pendingAction === `activate-${term.id}`;
                                    const isDeleting = pendingAction === `delete-${term.id}`;
                                    return (
                                        <tr key={term.id}>
                                            <td className="px-4 py-3">
                                                <div className="font-medium text-slate-900">
                                                    {term.name}
                                                </div>
                                                <div className="text-xs text-slate-500">
                                                    编号：{term.code}
                                                </div>
                                            </td>
                                            <td className="px-4 py-3 text-slate-700">
                                                <div>{term.startDate}</div>
                                                <div className="text-xs text-slate-500">
                                                    至 {term.endDate}
                                                </div>
                                            </td>
                                            <td className="px-4 py-3 text-slate-700">
                                                <div>{term.enrollmentStart}</div>
                                                <div className="text-xs text-slate-500">
                                                    截止 {term.enrollmentEnd}
                                                </div>
                                            </td>
                                            <td className="px-4 py-3">
                                                <div className="flex items-center justify-end gap-2">
                                                    {term.isActive ? (
                                                        <span className="inline-flex items-center rounded-full bg-emerald-100 px-3 py-1 text-xs font-semibold text-emerald-700">
                                                            当前学期
                                                        </span>
                                                    ) : (
                                                        <button
                                                            type="button"
                                                            onClick={() => handleActivate(term.id)}
                                                            disabled={isActivating}
                                                            className="rounded-full border border-indigo-200 px-3 py-1 text-xs font-medium text-indigo-600 transition hover:bg-indigo-50 disabled:cursor-not-allowed disabled:text-slate-400"
                                                        >
                                                            {isActivating ? "切换中..." : "设为当前"}
                                                        </button>
                                                    )}
                                                    <button
                                                        type="button"
                                                        onClick={() => handleEdit(term)}
                                                        className="text-xs font-medium text-slate-600 transition hover:text-slate-900"
                                                    >
                                                        编辑
                                                    </button>
                                                    <button
                                                        type="button"
                                                        onClick={() => handleDelete(term)}
                                                        disabled={isDeleting}
                                                        className="text-xs font-medium text-rose-500 transition hover:text-rose-600 disabled:cursor-not-allowed disabled:text-slate-400"
                                                    >
                                                        {isDeleting ? "删除中..." : "删除"}
                                                    </button>
                                                </div>
                                            </td>
                                        </tr>
                                    );
                                })
                            )}
                        </tbody>
                    </table>
                </div>
            </section>

            <section className="rounded-xl border border-slate-200 bg-white p-6">
                <div className="flex flex-wrap items-center justify-between gap-2">
                    <div>
                        <h3 className="text-lg font-semibold text-slate-900">
                            {isEditing ? "编辑学期" : "新增学期"}
                        </h3>
                        <p className="text-sm text-slate-600">
                            维护学期基础信息，可同时决定是否立即设为当前学期。
                        </p>
                    </div>
                    {isEditing && (
                        <button
                            type="button"
                            onClick={resetForm}
                            className="text-sm font-medium text-slate-500 underline-offset-4 hover:text-slate-900 hover:underline"
                        >
                            取消编辑
                        </button>
                    )}
                </div>
                <form className="mt-6 space-y-4" onSubmit={handleSubmit}>
                    <div className="grid gap-4 sm:grid-cols-2">
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">学期编号</label>
                            <input
                                type="text"
                                value={formState.code}
                                onChange={(event) => handleInputChange("code", event.target.value)}
                                placeholder="例如：2025-SPRING"
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">学期名称</label>
                            <input
                                type="text"
                                value={formState.name}
                                onChange={(event) => handleInputChange("name", event.target.value)}
                                placeholder="2025 春季学期"
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                    </div>
                    <div className="grid gap-4 sm:grid-cols-2">
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">开始日期</label>
                            <input
                                type="date"
                                value={formState.startDate}
                                onChange={(event) =>
                                    handleInputChange("startDate", event.target.value)
                                }
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">结束日期</label>
                            <input
                                type="date"
                                value={formState.endDate}
                                onChange={(event) =>
                                    handleInputChange("endDate", event.target.value)
                                }
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                    </div>
                    <div className="grid gap-4 sm:grid-cols-2">
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">报名开始</label>
                            <input
                                type="date"
                                value={formState.enrollmentStart}
                                onChange={(event) =>
                                    handleInputChange("enrollmentStart", event.target.value)
                                }
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                        <div className="space-y-1">
                            <label className="text-sm font-medium text-slate-700">报名截止</label>
                            <input
                                type="date"
                                value={formState.enrollmentEnd}
                                onChange={(event) =>
                                    handleInputChange("enrollmentEnd", event.target.value)
                                }
                                className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:ring-indigo-500"
                            />
                        </div>
                    </div>
                    <div className="flex items-center gap-3 rounded-lg border border-slate-200 bg-slate-50 px-4 py-3">
                        <input
                            id="term-is-active"
                            type="checkbox"
                            checked={formState.isActive}
                            onChange={(event) => handleInputChange("isActive", event.target.checked)}
                            className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                        />
                        <label htmlFor="term-is-active" className="text-sm text-slate-700">
                            创建/保存后设为当前学期
                        </label>
                    </div>
                    <div className="flex flex-wrap items-center gap-3">
                        <button
                            type="submit"
                            disabled={pendingAction === "save"}
                            className="inline-flex items-center rounded-md bg-indigo-600 px-5 py-2 text-sm font-semibold text-white transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-300"
                        >
                            {pendingAction === "save" ? "保存中..." : currentActionLabel}
                        </button>
                        {isEditing && (
                            <span className="text-sm text-slate-500">
                                当前编辑：{formState.name || "未命名"}
                            </span>
                        )}
                    </div>
                </form>
            </section>
        </div>
    );
}
