"use client";

import {
    Dispatch,
    FormEvent,
    SetStateAction,
    useCallback,
    useMemo,
    useState,
} from "react";

import { BulkAssignmentForm } from "@/components/forms/BulkAssignmentForm";
import type { ClassInstance, EnrollmentSummaryRow, PendingEnrollment } from "@/lib/types";
import { formatEnrollmentStatus, formatWeekday } from "@/lib/utils";
import { fetchEnrollmentSlotDetails } from "@/services/enrollmentService";
import {
    assignStudentsToClass,
    createClass,
    fetchClassesForSlot,
    updateClass,
} from "@/services/classAssignmentService";

interface Option {
    value: string;
    label: string;
}

interface Props {
    summaryRows: EnrollmentSummaryRow[];
}

export function ClassAssignmentBoard({ summaryRows }: Props) {
    const [selectedCampus, setSelectedCampus] = useState(() => summaryRows[0]?.campusId ?? "");
    const [selectedClub, setSelectedClub] = useState(() => summaryRows[0]?.clubId ?? "");
    const [selectedWeekday, setSelectedWeekday] = useState(
        () => (summaryRows[0] ? String(summaryRows[0].requestedWeekday) : ""),
    );
    const [students, setStudents] = useState<PendingEnrollment[]>([]);
    const [classes, setClasses] = useState<ClassInstance[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [hasFetched, setHasFetched] = useState(false);
    const [multiSelectMode, setMultiSelectMode] = useState(false);
    const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
    const [singleUpdatingId, setSingleUpdatingId] = useState<string | null>(null);
    const [bulkUpdating, setBulkUpdating] = useState(false);
    const [savingClass, setSavingClass] = useState(false);
    const [classFormError, setClassFormError] = useState<string | null>(null);
    const [editingClassId, setEditingClassId] = useState<string | null>(null);
    const createEmptyClassForm = () => ({
        classCode: "",
        startTime: "16:00",
        endTime: "17:30",
        location: "",
        capacity: "",
    });
    const [classForm, setClassForm] = useState(createEmptyClassForm);

    const campusOptions = useMemo(() => dedupeCampuses(summaryRows), [summaryRows]);
    const clubOptions = useMemo(
        () => dedupeClubs(summaryRows, selectedCampus),
        [summaryRows, selectedCampus],
    );
    const weekdayOptions = useMemo(
        () => collectWeekdays(summaryRows, selectedCampus, selectedClub),
        [summaryRows, selectedCampus, selectedClub],
    );

    const hasSelection = Boolean(selectedCampus && selectedClub && selectedWeekday);
    const selectedWeekdayNumber = selectedWeekday ? Number(selectedWeekday) : 0;
    const selectedSummary = summaryRows.find(
        (row) =>
            row.campusId === selectedCampus &&
            row.clubId === selectedClub &&
            String(row.requestedWeekday) === selectedWeekday,
    );

    const resetClassForm = () => {
        setClassForm(createEmptyClassForm());
        setEditingClassId(null);
        setClassFormError(null);
    };

    const resetWorkspace = () => {
        setStudents([]);
        setClasses([]);
        setSelectedIds(new Set());
        setMultiSelectMode(false);
        setHasFetched(false);
        setError(null);
        resetClassForm();
    };

    const handleCampusChange = (value: string) => {
        setSelectedCampus(value);
        const nextClubs = dedupeClubs(summaryRows, value);
        const nextClubId = nextClubs[0]?.value ?? "";
        setSelectedClub(nextClubId);
        const nextWeekdays = collectWeekdays(summaryRows, value, nextClubId);
        setSelectedWeekday(nextWeekdays[0]?.value ?? "");
        resetWorkspace();
    };

    const handleClubChange = (value: string) => {
        setSelectedClub(value);
        const nextWeekdays = collectWeekdays(summaryRows, selectedCampus, value);
        setSelectedWeekday(nextWeekdays[0]?.value ?? "");
        resetWorkspace();
    };

    const handleWeekdayChange = (value: string) => {
        setSelectedWeekday(value);
        resetWorkspace();
    };

    const sortClasses = useCallback((list: ClassInstance[]) => {
        return [...list].sort((a, b) => a.classCode.localeCompare(b.classCode, "zh-CN"));
    }, []);

    const loadSlotData = async () => {
        if (!hasSelection) {
            return;
        }
        setLoading(true);
        setError(null);
        setSelectedIds(new Set());
        setMultiSelectMode(false);
        try {
            const [slotStudents, slotClasses] = await Promise.all([
                fetchEnrollmentSlotDetails({
                    campusId: selectedCampus,
                    clubId: selectedClub,
                    weekday: selectedWeekdayNumber,
                }),
                fetchClassesForSlot({
                    campusId: selectedCampus,
                    clubId: selectedClub,
                    weekday: selectedWeekdayNumber,
                }),
            ]);
            setStudents(slotStudents);
            setClasses(sortClasses(slotClasses));
            setHasFetched(true);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            setStudents([]);
            setClasses([]);
            setHasFetched(false);
        } finally {
            setLoading(false);
        }
    };

    const handleEditClass = (cls: ClassInstance) => {
        setClassForm({
            classCode: cls.classCode,
            startTime: cls.startTime,
            endTime: cls.endTime,
            location: cls.location ?? "",
            capacity: cls.capacity ? String(cls.capacity) : "",
        });
        setEditingClassId(cls.id);
        setClassFormError(null);
    };

    const handleCancelEdit = () => {
        resetClassForm();
    };

    const toggleSelection = (id: string) => {
        setSelectedIds((current) => {
            const next = new Set(current);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
    };

    const applyAssignmentLocally = useCallback(
        (targetIds: string[], nextClassId: string | null) => {
            if (targetIds.length === 0) {
                return;
            }
            const targetSet = new Set(targetIds);
            const deltas = new Map<string, number>();
            students.forEach((student) => {
                if (!targetSet.has(student.enrollmentId)) {
                    return;
                }
                const previous = student.classId ?? null;
                if (previous && previous !== nextClassId) {
                    deltas.set(previous, (deltas.get(previous) ?? 0) - 1);
                }
                if (nextClassId && previous !== nextClassId) {
                    deltas.set(nextClassId, (deltas.get(nextClassId) ?? 0) + 1);
                }
            });

            const targetClass = nextClassId
                ? classes.find((cls) => cls.id === nextClassId)
                : undefined;

            setClasses((current) =>
                current.map((cls) => {
                    const delta = deltas.get(cls.id);
                    if (!delta) {
                        return cls;
                    }
                    return {
                        ...cls,
                        assignedCount: Math.max(0, cls.assignedCount + delta),
                    };
                }),
            );

            setStudents((current) =>
                current.map((student) => {
                    if (!targetSet.has(student.enrollmentId)) {
                        return student;
                    }
                    return {
                        ...student,
                        status: nextClassId ? "ACTIVE" : "PENDING",
                        classId: nextClassId ?? undefined,
                        classCode: nextClassId ? targetClass?.classCode ?? "" : undefined,
                    };
                }),
            );
        },
        [classes, students],
    );

    const handleSingleAssign = async (enrollmentId: string, value: string) => {
        if (!hasSelection) {
            return;
        }
        const targetClassId = value.length > 0 ? value : null;
        setSingleUpdatingId(enrollmentId);
        setError(null);
        try {
            await assignStudentsToClass({
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
                classId: targetClassId,
                enrollmentIds: [enrollmentId],
            });
            applyAssignmentLocally([enrollmentId], targetClassId);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
        } finally {
            setSingleUpdatingId(null);
        }
    };

    const handleBulkApply = async (targetClassId: string | null) => {
        if (!hasSelection || selectedIds.size === 0) {
            return;
        }
        setBulkUpdating(true);
        setError(null);
        const target = Array.from(selectedIds);
        try {
            await assignStudentsToClass({
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
                classId: targetClassId,
                enrollmentIds: target,
            });
            applyAssignmentLocally(target, targetClassId);
            setSelectedIds(new Set());
            setMultiSelectMode(false);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
        } finally {
            setBulkUpdating(false);
        }
    };

    const handleSubmitClassForm = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        if (!hasSelection) {
            setClassFormError("请先选择校区 / 社团 / 星期后再维护班级");
            return;
        }
        if (!classForm.classCode.trim()) {
            setClassFormError("班级名称不能为空");
            return;
        }
        const capacityNumber =
            classForm.capacity.trim().length > 0 ? Number(classForm.capacity.trim()) : undefined;
        if (capacityNumber !== undefined && Number.isNaN(capacityNumber)) {
            setClassFormError("班级容量需为数字");
            return;
        }

        setSavingClass(true);
        setClassFormError(null);
        try {
            const payload = {
                campusId: selectedCampus,
                clubId: selectedClub,
                weekday: selectedWeekdayNumber,
                classCode: classForm.classCode.trim(),
                startTime: classForm.startTime,
                endTime: classForm.endTime,
                location: classForm.location.trim() || undefined,
                capacity: capacityNumber,
            };
            if (editingClassId) {
                const updated = await updateClass({
                    ...payload,
                    id: editingClassId,
                });
                setClasses((current) =>
                    sortClasses(current.map((cls) => (cls.id === updated.id ? updated : cls))),
                );
            } else {
                const created = await createClass(payload);
                setClasses((current) => sortClasses([...current, created]));
            }
            resetClassForm();
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setClassFormError(message);
        } finally {
            setSavingClass(false);
        }
    };

    if (summaryRows.length === 0) {
        return (
            <p className="rounded-lg bg-slate-50 px-4 py-3 text-sm text-slate-500">
                暂无报名数据，待导入学生后再进行分班。
            </p>
        );
    }

    return (
        <div className="space-y-6 rounded-xl border border-slate-200 p-4">
            <div className="space-y-2">
                <h3 className="text-lg font-semibold text-slate-900">选择社团并开始分班</h3>
                <p className="text-sm text-slate-500">
                    选择校区 / 社团 / 星期后点击“开始分班”，系统会拉取实时报名名单及已有班级配置。
                </p>
            </div>
            <div className="grid gap-4 md:grid-cols-3">
                <FilterControl
                    label="校区"
                    value={selectedCampus}
                    onChange={handleCampusChange}
                    options={campusOptions}
                />
                <FilterControl
                    label="社团"
                    value={selectedClub}
                    onChange={handleClubChange}
                    options={clubOptions}
                />
                <FilterControl
                    label="星期"
                    value={selectedWeekday}
                    onChange={handleWeekdayChange}
                    options={weekdayOptions}
                />
            </div>
            <div className="flex flex-wrap items-center gap-3">
                <button
                    type="button"
                    className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-300"
                    onClick={loadSlotData}
                    disabled={!hasSelection || loading}
                >
                    {loading ? "加载中..." : hasFetched ? "重新加载" : "开始分班"}
                </button>
                {hasSelection ? (
                    <span className="text-sm text-slate-500">
                        当前组合：{selectedSummary?.campusName ?? "--"} /{" "}
                        {selectedSummary?.clubName ?? "--"} / {formatWeekday(selectedWeekdayNumber)}
                    </span>
                ) : (
                    <span className="text-sm text-slate-500">请选择全部筛选条件后再操作。</span>
                )}
            </div>
            {error ? (
                <div className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-600">{error}</div>
            ) : null}
            {hasFetched && (
                <div className="space-y-6">
                    <ClassListPanel
                        classes={classes}
                        classForm={classForm}
                        classFormError={classFormError}
                        editingClassId={editingClassId}
                        saving={savingClass}
                        onFormChange={setClassForm}
                        onSubmit={handleSubmitClassForm}
                        onEditClass={handleEditClass}
                        onCancelEdit={handleCancelEdit}
                    />
                    <StudentAssignmentTable
                        students={students}
                        classes={classes}
                        multiSelectMode={multiSelectMode}
                        selectedIds={selectedIds}
                        onToggleSelect={toggleSelection}
                        onToggleMode={() => {
                            setMultiSelectMode((mode) => {
                                const next = !mode;
                                if (!next) {
                                    setSelectedIds(new Set());
                                }
                                return next;
                            });
                        }}
                        onSingleAssign={handleSingleAssign}
                        singleUpdatingId={singleUpdatingId}
                        loading={loading}
                    />
                    {multiSelectMode && (
                        <BulkAssignmentForm
                            classes={classes}
                            selectedCount={selectedIds.size}
                            onApply={handleBulkApply}
                            disabled={bulkUpdating}
                        />
                    )}
                </div>
            )}
        </div>
    );
}

function FilterControl({
    label,
    value,
    onChange,
    options,
}: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    options: Option[];
}) {
    return (
        <label className="flex flex-col gap-1 text-sm">
            <span className="text-slate-600">{label}</span>
            <select
                value={value}
                onChange={(event) => onChange(event.target.value)}
                className="rounded-lg border border-slate-200 px-3 py-2 text-slate-900 focus:border-indigo-500 focus:outline-none"
            >
                {options.map((option) => (
                    <option key={option.value} value={option.value}>
                        {option.label}
                    </option>
                ))}
            </select>
        </label>
    );
}

function ClassListPanel({
    classes,
    classForm,
    classFormError,
    editingClassId,
    saving,
    onFormChange,
    onSubmit,
    onEditClass,
    onCancelEdit,
}: {
    classes: ClassInstance[];
    classForm: {
        classCode: string;
        startTime: string;
        endTime: string;
        location: string;
        capacity: string;
    };
    classFormError: string | null;
    editingClassId: string | null;
    saving: boolean;
    onFormChange: Dispatch<
        SetStateAction<{
            classCode: string;
            startTime: string;
            endTime: string;
            location: string;
            capacity: string;
        }>
    >;
    onSubmit: (event: FormEvent<HTMLFormElement>) => void;
    onEditClass: (cls: ClassInstance) => void;
    onCancelEdit: () => void;
}) {
    const isEditing = Boolean(editingClassId);
    return (
        <div className="space-y-3 rounded-xl border border-dashed border-slate-200 p-4">
            <div className="flex flex-col gap-1">
                <h4 className="text-base font-semibold text-slate-900">班级配置</h4>
                <p className="text-xs text-slate-500">
                    维护上课时间、地点并实时查看已分配人数。
                    {isEditing ? " 当前处于编辑模式，修改完成后请保存或取消。" : ""}
                </p>
            </div>
            {classes.length === 0 ? (
                <p className="text-sm text-slate-500">当前社团暂无班级，请先创建。</p>
            ) : (
                <div className="grid gap-3 md:grid-cols-2">
                    {classes.map((cls) => (
                        <div
                            key={cls.id}
                            className={`rounded-lg border p-3 shadow-sm ${
                                editingClassId === cls.id ? "border-indigo-200 bg-indigo-50/40" : "border-slate-100"
                            }`}
                        >
                            <div className="flex items-center justify-between">
                                <p className="text-sm font-semibold text-slate-900">{cls.classCode}</p>
                                <span className="text-xs text-slate-500">
                                    已分配 {cls.assignedCount}
                                    {cls.capacity ? ` / ${cls.capacity}` : ""}
                                </span>
                            </div>
                            <p className="text-sm text-slate-600">
                                {cls.startTime} - {cls.endTime} · {cls.location ?? "地点待定"}
                            </p>
                            <button
                                type="button"
                                onClick={() => onEditClass(cls)}
                                className="mt-2 text-xs font-medium text-indigo-600 hover:text-indigo-500"
                            >
                                {editingClassId === cls.id ? "编辑中..." : "编辑"}
                            </button>
                        </div>
                    ))}
                </div>
            )}
            <form className="space-y-3 rounded-lg bg-slate-50 p-3" onSubmit={onSubmit}>
            <div className="grid gap-3 md:grid-cols-2">
                    <label className="text-sm text-slate-600">
                        班级名称
                        <input
                            required
                            value={classForm.classCode}
                            onChange={(event) =>
                                onFormChange((current) => ({ ...current, classCode: event.target.value }))
                            }
                            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                            placeholder="例如 机器人一班"
                        />
                    </label>
                    <label className="text-sm text-slate-600">
                        上课地点
                        <input
                            value={classForm.location}
                            onChange={(event) =>
                                onFormChange((current) => ({ ...current, location: event.target.value }))
                            }
                            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                            placeholder="例如 科技楼301"
                        />
                    </label>
                </div>
                <div className="grid gap-3 md:grid-cols-3">
                    <label className="text-sm text-slate-600">
                        开始时间
                        <input
                            type="time"
                            value={classForm.startTime}
                            onChange={(event) =>
                                onFormChange((current) => ({ ...current, startTime: event.target.value }))
                            }
                            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                        />
                    </label>
                    <label className="text-sm text-slate-600">
                        结束时间
                        <input
                            type="time"
                            value={classForm.endTime}
                            onChange={(event) =>
                                onFormChange((current) => ({ ...current, endTime: event.target.value }))
                            }
                            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                        />
                    </label>
                    <label className="text-sm text-slate-600">
                        容量（选填）
                        <input
                            type="number"
                            min={1}
                            value={classForm.capacity}
                            onChange={(event) =>
                                onFormChange((current) => ({ ...current, capacity: event.target.value }))
                            }
                            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                        />
                    </label>
                </div>
                {classFormError ? <p className="text-sm text-rose-600">{classFormError}</p> : null}
                <div className="flex flex-wrap gap-3">
                    <button
                        type="submit"
                        disabled={saving}
                        className="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-400"
                    >
                        {saving ? "保存中..." : isEditing ? "保存班级" : "新建班级"}
                    </button>
                    {isEditing && (
                        <button
                            type="button"
                            onClick={onCancelEdit}
                            className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 transition hover:bg-white"
                        >
                            取消编辑
                        </button>
                    )}
                </div>
            </form>
        </div>
    );
}

function StudentAssignmentTable({
    students,
    classes,
    multiSelectMode,
    selectedIds,
    onToggleSelect,
    onToggleMode,
    onSingleAssign,
    singleUpdatingId,
    loading,
}: {
    students: PendingEnrollment[];
    classes: ClassInstance[];
    multiSelectMode: boolean;
    selectedIds: Set<string>;
    onToggleSelect: (id: string) => void;
    onToggleMode: () => void;
    onSingleAssign: (enrollmentId: string, classId: string) => Promise<void>;
    singleUpdatingId: string | null;
    loading: boolean;
}) {
    const options = [
        { value: "", label: "待定班（未分配）" },
        ...classes.map((cls) => ({
            value: cls.id,
            label: `${cls.classCode}（${cls.startTime}-${cls.endTime}）`,
        })),
    ];

    return (
        <div className="space-y-3 rounded-xl border border-slate-200 p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <h4 className="text-base font-semibold text-slate-900">学生名单</h4>
                    <p className="text-xs text-slate-500">
                        直接在下拉框中选择班级即可实时写入后端。
                    </p>
                </div>
                <button
                    type="button"
                    onClick={onToggleMode}
                    className="rounded-md border border-slate-300 px-3 py-1 text-sm font-medium text-slate-700 transition hover:bg-slate-50"
                >
                    {multiSelectMode ? "取消多选" : "多选批量分班"}
                </button>
            </div>
            {students.length === 0 ? (
                <p className="text-sm text-slate-500">
                    {loading ? "加载中..." : "暂无报名学生，或均已退课。"}
                </p>
            ) : (
                <div className="overflow-x-auto">
                    <table className="min-w-full rounded-xl border border-slate-200 text-sm">
                        <thead className="bg-slate-50 text-left text-slate-500">
                            <tr>
                                {multiSelectMode && <th className="px-4 py-2">选择</th>}
                                <th className="px-4 py-2">学生姓名</th>
                                <th className="px-4 py-2">所在班级</th>
                                <th className="px-4 py-2">报名状态</th>
                                <th className="px-4 py-2">分班</th>
                            </tr>
                        </thead>
                        <tbody>
                            {students.map((student) => (
                                <tr key={student.enrollmentId} className="border-t">
                                    {multiSelectMode && (
                                        <td className="px-4 py-2">
                                            <input
                                                type="checkbox"
                                                checked={selectedIds.has(student.enrollmentId)}
                                                onChange={() => onToggleSelect(student.enrollmentId)}
                                                className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                                            />
                                        </td>
                                    )}
                                    <td className="px-4 py-2 font-medium text-slate-900">
                                        {student.studentName}
                                    </td>
                                    <td className="px-4 py-2 text-slate-600">{student.homeroom}</td>
                                    <td className="px-4 py-2 text-slate-600">
                                        {formatEnrollmentStatus(student.status)}
                                    </td>
                                    <td className="px-4 py-2">
                                        <select
                                            value={student.classId ?? ""}
                                            onChange={(event) =>
                                                onSingleAssign(student.enrollmentId, event.target.value)
                                            }
                                            disabled={multiSelectMode || singleUpdatingId === student.enrollmentId}
                                            className="w-full rounded-lg border border-slate-200 px-3 py-1.5 text-slate-900 focus:border-indigo-500 focus:outline-none disabled:opacity-60"
                                        >
                                            {options.map((option) => (
                                                <option key={option.value} value={option.value}>
                                                    {option.label}
                                                </option>
                                            ))}
                                        </select>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
}

function dedupeCampuses(rows: EnrollmentSummaryRow[]): Option[] {
    const seen = new Map<string, string>();
    rows.forEach((row) => {
        if (!seen.has(row.campusId)) {
            seen.set(row.campusId, row.campusName);
        }
    });
    return Array.from(seen.entries()).map(([value, label]) => ({ value, label }));
}

function dedupeClubs(rows: EnrollmentSummaryRow[], campusId: string): Option[] {
    const seen = new Map<string, string>();
    rows.forEach((row) => {
        if (campusId && row.campusId !== campusId) {
            return;
        }
        if (!seen.has(row.clubId)) {
            seen.set(row.clubId, row.clubName);
        }
    });
    return Array.from(seen.entries()).map(([value, label]) => ({ value, label }));
}

function collectWeekdays(
    rows: EnrollmentSummaryRow[],
    campusId: string,
    clubId: string,
): Option[] {
    const seen = new Set<number>();
    rows.forEach((row) => {
        if (campusId && row.campusId !== campusId) {
            return;
        }
        if (clubId && row.clubId !== clubId) {
            return;
        }
        if (!seen.has(row.requestedWeekday)) {
            seen.add(row.requestedWeekday);
        }
    });
    return Array.from(seen.values()).map((value) => ({
        value: String(value),
        label: formatWeekday(value),
    }));
}
