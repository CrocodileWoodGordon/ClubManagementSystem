"use client";

import {
    Fragment,
    useCallback,
    useEffect,
    useMemo,
    useState,
} from "react";

import type { HomeroomRoster, RosterStudent } from "@/lib/types";
import {
    StudentRosterServiceError,
    CloneRosterSummary,
    cloneRoster,
    createStudent,
    deleteStudent,
    fetchHomeroomStudents,
    fetchHomerooms,
    updateHomeroom,
    updateStudent,
} from "@/services/studentRosterService";

export interface TermOption {
    id: string;
    code: string;
    name: string;
    isActive: boolean;
}

export interface CampusOption {
    id: string;
    name: string;
    shortName?: string;
}

interface StudentRosterWorkspaceProps {
    terms: TermOption[];
    campuses: CampusOption[];
    defaultTermId?: string;
    refreshToken: number;
}

interface HomeroomFormState {
    displayName: string;
    headTeacherName: string;
    headTeacherPhone: string;
    notes: string;
}

interface StudentFormState {
    fullName: string;
    studentCode: string;
    primaryGuardianName: string;
    primaryGuardianPhone: string;
    isTeacherChild: boolean;
}

export function StudentRosterWorkspace({
    terms,
    campuses,
    defaultTermId,
    refreshToken,
}: StudentRosterWorkspaceProps) {
    const firstTermId = defaultTermId ?? terms[0]?.id ?? "";
    const [selectedTermId, setSelectedTermId] = useState(firstTermId);
    const [selectedCampusId, setSelectedCampusId] = useState(campuses[0]?.id ?? "");
    const [searchInput, setSearchInput] = useState("");
    const [appliedSearch, setAppliedSearch] = useState("");
    const [homerooms, setHomerooms] = useState<HomeroomRoster[]>([]);
    const [homeroomsLoading, setHomeroomsLoading] = useState(false);
    const [homeroomListError, setHomeroomListError] = useState<string | null>(null);
    const [selectedHomeroomId, setSelectedHomeroomId] = useState<string>("");
    const [students, setStudents] = useState<RosterStudent[]>([]);
    const [studentsLoading, setStudentsLoading] = useState(false);
    const [studentsError, setStudentsError] = useState<string | null>(null);
    const [homeroomForm, setHomeroomForm] = useState<HomeroomFormState>(createHomeroomForm());
    const [savingHomeroom, setSavingHomeroom] = useState(false);
    const [homeroomMessage, setHomeroomMessage] = useState<string | null>(null);
    const [homeroomFormError, setHomeroomFormError] = useState<string | null>(null);
    const [studentFormVisible, setStudentFormVisible] = useState(false);
    const [editingStudentId, setEditingStudentId] = useState<string | null>(null);
    const [studentForm, setStudentForm] = useState<StudentFormState>(createStudentForm());
    const [studentFormError, setStudentFormError] = useState<string | null>(null);
    const [studentSaving, setStudentSaving] = useState(false);
    const [clonePanelOpen, setClonePanelOpen] = useState(false);
    const [cloneSourceTermId, setCloneSourceTermId] = useState<string>(
        () => terms.find((term) => term.id !== firstTermId)?.id ?? "",
    );
    const [cloneError, setCloneError] = useState<string | null>(null);
    const [cloneStatus, setCloneStatus] = useState<CloneRosterSummary | null>(null);
    const [cloning, setCloning] = useState(false);

    const selectedHomeroom = useMemo(
        () => homerooms.find((item) => item.id === selectedHomeroomId),
        [homerooms, selectedHomeroomId],
    );

    const cloneTermOptions = useMemo(
        () => terms.filter((term) => term.id !== selectedTermId),
        [terms, selectedTermId],
    );

    const loadHomerooms = useCallback(async () => {
        if (!selectedTermId || !selectedCampusId) {
            setHomerooms([]);
            setSelectedHomeroomId("");
            return;
        }
        setHomeroomsLoading(true);
        setHomeroomListError(null);
        try {
            const data = await fetchHomerooms({
                termId: selectedTermId,
                campusId: selectedCampusId,
                search: appliedSearch,
            });
            setHomerooms(data);
            if (data.length === 0) {
                setSelectedHomeroomId("");
                setStudents([]);
                setStudentsError(null);
            } else if (!data.some((item) => item.id === selectedHomeroomId)) {
                setSelectedHomeroomId(data[0].id);
            }
        } catch (error) {
            setHomeroomListError(extractError(error));
            setHomerooms([]);
            setSelectedHomeroomId("");
            setStudents([]);
        } finally {
            setHomeroomsLoading(false);
        }
    }, [selectedTermId, selectedCampusId, appliedSearch, selectedHomeroomId]);

    useEffect(() => {
        loadHomerooms();
    }, [loadHomerooms, refreshToken]);

    const loadStudents = useCallback(
        async (homeroomId: string) => {
            if (!homeroomId) {
                setStudents([]);
                return;
            }
            setStudentsLoading(true);
            setStudentsError(null);
            try {
                const data = await fetchHomeroomStudents(homeroomId, {
                    termId: selectedTermId,
                });
                setStudents([...data].sort(sortByName));
            } catch (error) {
                setStudentsError(extractError(error));
                setStudents([]);
            } finally {
                setStudentsLoading(false);
            }
        },
        [selectedTermId],
    );

    useEffect(() => {
        if (selectedHomeroomId) {
            loadStudents(selectedHomeroomId);
        } else {
            setStudents([]);
        }
    }, [selectedHomeroomId, loadStudents]);

    useEffect(() => {
        if (selectedHomeroom) {
            setHomeroomForm({
                displayName: selectedHomeroom.displayName,
                headTeacherName: selectedHomeroom.headTeacherName ?? "",
                headTeacherPhone: selectedHomeroom.headTeacherPhone ?? "",
                notes: selectedHomeroom.notes ?? "",
            });
            setHomeroomMessage(null);
        } else {
            setHomeroomForm(createHomeroomForm());
        }
    }, [selectedHomeroom]);

    const handleApplySearch = () => {
        setAppliedSearch(searchInput.trim());
    };

    const handleClearSearch = () => {
        setSearchInput("");
        setAppliedSearch("");
    };

    const handleSaveHomeroom = async () => {
        if (!selectedHomeroom) {
            return;
        }
        setSavingHomeroom(true);
        setHomeroomMessage(null);
        setHomeroomFormError(null);
        try {
            const updated = await updateHomeroom(selectedHomeroom.id, {
                termId: selectedTermId,
                displayName: homeroomForm.displayName,
                headTeacherName: homeroomForm.headTeacherName,
                headTeacherPhone: homeroomForm.headTeacherPhone,
                notes: homeroomForm.notes,
            });
            setHomeroomMessage("班级信息已更新");
            setHomerooms((list) =>
                list.map((item) => (item.id === updated.id ? updated : item)),
            );
        } catch (error) {
            setHomeroomFormError(extractError(error));
        } finally {
            setSavingHomeroom(false);
        }
    };

    const handleStartCreateStudent = () => {
        setEditingStudentId(null);
        setStudentForm(createStudentForm());
        setStudentFormVisible(true);
        setStudentFormError(null);
    };

    const handleStartEditStudent = (student: RosterStudent) => {
        setEditingStudentId(student.id);
        setStudentForm({
            fullName: student.fullName,
            studentCode: student.studentCode ?? "",
            primaryGuardianName: student.primaryGuardianName ?? "",
            primaryGuardianPhone: student.primaryGuardianPhone ?? "",
            isTeacherChild: student.isTeacherChild,
        });
        setStudentFormVisible(true);
        setStudentFormError(null);
    };

    const handleCancelStudentForm = () => {
        setStudentFormVisible(false);
        setEditingStudentId(null);
        setStudentFormError(null);
    };

    const handleSubmitStudentForm = async () => {
        if (!selectedHomeroom) {
            return;
        }
        if (studentForm.fullName.trim().length === 0) {
            setStudentFormError("请填写学生姓名");
            return;
        }
        setStudentSaving(true);
        setStudentFormError(null);
        try {
            let result: RosterStudent;
            if (editingStudentId) {
                result = await updateStudent(editingStudentId, {
                    termId: selectedTermId,
                    homeroomId: selectedHomeroom.id,
                    fullName: studentForm.fullName,
                    studentCode: studentForm.studentCode,
                    primaryGuardianName: studentForm.primaryGuardianName,
                    primaryGuardianPhone: studentForm.primaryGuardianPhone,
                    isTeacherChild: studentForm.isTeacherChild,
                });
                setStudents((list) =>
                    list.map((item) => (item.id === result.id ? result : item)),
                );
            } else {
                result = await createStudent(selectedHomeroom.id, {
                    termId: selectedTermId,
                    fullName: studentForm.fullName,
                    studentCode: studentForm.studentCode,
                    primaryGuardianName: studentForm.primaryGuardianName,
                    primaryGuardianPhone: studentForm.primaryGuardianPhone,
                    isTeacherChild: studentForm.isTeacherChild,
                });
                setStudents((list) => [...list, result].sort(sortByName));
                adjustHomeroomCount(selectedHomeroom.id, 1);
            }
            setStudentFormVisible(false);
            setEditingStudentId(null);
            setStudentForm(createStudentForm());
        } catch (error) {
            setStudentFormError(extractError(error));
        } finally {
            setStudentSaving(false);
        }
    };

    const handleDeleteStudent = async (studentId: string) => {
        if (!selectedHomeroom) {
            return;
        }
        const confirmed = window.confirm("确定要删除该学生吗？此操作可在 Excel 导入后重新添加。");
        if (!confirmed) {
            return;
        }
        setStudentsError(null);
        try {
            await deleteStudent(studentId, { termId: selectedTermId });
            setStudents((list) => list.filter((item) => item.id !== studentId));
            adjustHomeroomCount(selectedHomeroom.id, -1);
        } catch (error) {
            setStudentsError(extractError(error));
        }
    };

    const adjustHomeroomCount = (homeroomId: string, delta: number) => {
        setHomerooms((list) =>
            list.map((item) =>
                item.id === homeroomId
                    ? { ...item, studentCount: Math.max(0, item.studentCount + delta) }
                    : item,
            ),
        );
    };

    const handleCloneRoster = async () => {
        if (!cloneSourceTermId || !selectedTermId || !selectedCampusId) {
            return;
        }
        setCloning(true);
        setCloneError(null);
        setCloneStatus(null);
        try {
            const result = await cloneRoster({
                sourceTermId: cloneSourceTermId,
                targetTermId: selectedTermId,
                campusId: selectedCampusId,
            });
            setCloneStatus(result);
            setClonePanelOpen(false);
            await loadHomerooms();
        } catch (error) {
            setCloneError(extractError(error));
        } finally {
            setCloning(false);
        }
    };

    if (terms.length === 0 || campuses.length === 0) {
        return <p className="text-sm text-slate-600">请先在“设置”中创建学期与校区。</p>;
    }

    return (
        <div className="space-y-6">
            <div className="flex flex-col gap-4 lg:flex-row lg:items-end">
                <div className="flex flex-col gap-1">
                    <label className="text-sm font-medium text-slate-700">学期</label>
                    <select
                        value={selectedTermId}
                        onChange={(event) => {
                            setSelectedTermId(event.target.value);
                            setCloneSourceTermId(
                                terms.find((term) => term.id !== event.target.value)?.id ?? "",
                            );
                        }}
                        className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                    >
                        {terms.map((term) => (
                            <option key={term.id} value={term.id}>
                                {term.name} {term.isActive ? "（当前学期）" : ""}
                            </option>
                        ))}
                    </select>
                </div>
                <div className="flex flex-col gap-1">
                    <label className="text-sm font-medium text-slate-700">校区</label>
                    <select
                        value={selectedCampusId}
                        onChange={(event) => setSelectedCampusId(event.target.value)}
                        className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                    >
                        {campuses.map((campus) => (
                            <option key={campus.id} value={campus.id}>
                                {campus.name}
                            </option>
                        ))}
                    </select>
                </div>
                <div className="flex flex-1 flex-col gap-1">
                    <label className="text-sm font-medium text-slate-700">班级搜索</label>
                    <div className="flex gap-2">
                        <input
                            type="text"
                            value={searchInput}
                            onChange={(event) => setSearchInput(event.target.value)}
                            placeholder="按班级名称或年级筛选"
                            className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                        />
                        <button
                            type="button"
                            onClick={handleApplySearch}
                            className="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800"
                        >
                            查询
                        </button>
                        {appliedSearch && (
                            <button
                                type="button"
                                onClick={handleClearSearch}
                                className="rounded-lg border border-slate-200 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50"
                            >
                                清空
                            </button>
                        )}
                    </div>
                </div>
            </div>

            <div className="grid gap-6 lg:grid-cols-12">
                <aside className="space-y-4 rounded-2xl border border-slate-200 bg-white p-4 lg:col-span-4">
                    <div className="flex items-center justify-between">
                        <p className="text-sm font-medium text-slate-900">班级列表</p>
                        <button
                            type="button"
                            onClick={loadHomerooms}
                            className="text-xs text-indigo-600 hover:underline"
                        >
                            刷新
                        </button>
                    </div>
                    {homeroomsLoading ? (
                        <p className="text-sm text-slate-500">加载中...</p>
                    ) : homeroomListError ? (
                        <p className="text-sm text-red-600">{homeroomListError}</p>
                    ) : homerooms.length === 0 ? (
                        <div className="space-y-3 rounded-xl border border-dashed border-slate-300 p-4 text-sm text-slate-600">
                            <p>当前学期尚未导入学生名单，可通过导入或复用旧学期数据。</p>
                            {cloneTermOptions.length > 0 && (
                                <Fragment>
                                    <button
                                        type="button"
                                        onClick={() => setClonePanelOpen((prev) => !prev)}
                                        className="w-full rounded-lg bg-slate-900 px-3 py-2 text-sm font-medium text-white hover:bg-slate-800"
                                    >
                                        {clonePanelOpen ? "收起复用设置" : "从旧学期复用"}
                                    </button>
                                    {clonePanelOpen && (
                                        <div className="space-y-2 rounded-lg bg-slate-50 p-3">
                                            <label className="text-xs text-slate-600">选择来源学期</label>
                                            <select
                                                value={cloneSourceTermId}
                                                onChange={(event) => setCloneSourceTermId(event.target.value)}
                                                className="w-full rounded-lg border border-slate-200 px-2 py-1 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                            >
                                                {cloneTermOptions.map((term) => (
                                                    <option key={term.id} value={term.id}>
                                                        {term.name}
                                                    </option>
                                                ))}
                                            </select>
                                            <button
                                                type="button"
                                                onClick={handleCloneRoster}
                                                disabled={cloning || !cloneSourceTermId}
                                                className="w-full rounded-lg bg-indigo-600 px-3 py-2 text-sm font-medium text-white transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-indigo-300"
                                            >
                                                {cloning ? "复用中..." : "立即复用"}
                                            </button>
                                            {cloneError && <p className="text-xs text-red-600">{cloneError}</p>}
                                            {cloneStatus && (
                                                <p className="text-xs text-emerald-600">
                                                    已复制 {cloneStatus.copiedHomerooms} 个班级 /{" "}
                                                    {cloneStatus.copiedStudents} 名学生
                                                </p>
                                            )}
                                        </div>
                                    )}
                                </Fragment>
                            )}
                        </div>
                    ) : (
                        <ul className="space-y-2">
                            {homerooms.map((homeroom) => (
                                <li key={homeroom.id}>
                                    <button
                                        type="button"
                                        onClick={() => setSelectedHomeroomId(homeroom.id)}
                                        className={[
                                            "w-full rounded-xl border px-4 py-3 text-left transition",
                                            homeroom.id === selectedHomeroomId
                                                ? "border-indigo-500 bg-indigo-50 text-indigo-900"
                                                : "border-slate-200 bg-white text-slate-900 hover:border-slate-300",
                                        ].join(" ")}
                                    >
                                        <p className="text-sm font-semibold">{homeroom.displayName}</p>
                                        <p className="text-xs text-slate-500">
                                            {homeroom.gradeLabel} · {homeroom.classLabel}
                                        </p>
                                        <p className="text-xs text-slate-500">
                                            学生 {homeroom.studentCount} 人
                                        </p>
                                    </button>
                                </li>
                            ))}
                        </ul>
                    )}
                </aside>
                <main className="space-y-6 rounded-2xl border border-slate-200 bg-white p-6 lg:col-span-8">
                    {selectedHomeroom ? (
                        <Fragment>
                            <section className="space-y-4">
                                <div>
                                    <h3 className="text-lg font-semibold text-slate-900">
                                        {selectedHomeroom.displayName}
                                    </h3>
                                    <p className="text-sm text-slate-500">
                                        {selectedHomeroom.campusName} · {selectedHomeroom.academicYear} 学年
                                    </p>
                                </div>
                                <div className="grid gap-4 md:grid-cols-2">
                                    <label className="flex flex-col gap-1 text-sm text-slate-700">
                                        班级展示名
                                        <input
                                            type="text"
                                            value={homeroomForm.displayName}
                                            onChange={(event) =>
                                                setHomeroomForm((prev) => ({
                                                    ...prev,
                                                    displayName: event.target.value,
                                                }))
                                            }
                                            className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                        />
                                    </label>
                                    <label className="flex flex-col gap-1 text-sm text-slate-700">
                                        班主任
                                        <input
                                            type="text"
                                            value={homeroomForm.headTeacherName}
                                            onChange={(event) =>
                                                setHomeroomForm((prev) => ({
                                                    ...prev,
                                                    headTeacherName: event.target.value,
                                                }))
                                            }
                                            className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                        />
                                    </label>
                                    <label className="flex flex-col gap-1 text-sm text-slate-700">
                                        班主任电话
                                        <input
                                            type="text"
                                            value={homeroomForm.headTeacherPhone}
                                            onChange={(event) =>
                                                setHomeroomForm((prev) => ({
                                                    ...prev,
                                                    headTeacherPhone: event.target.value,
                                                }))
                                            }
                                            className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                        />
                                    </label>
                                    <label className="flex flex-col gap-1 text-sm text-slate-700 md:col-span-2">
                                        备注
                                        <textarea
                                            value={homeroomForm.notes}
                                            onChange={(event) =>
                                                setHomeroomForm((prev) => ({
                                                    ...prev,
                                                    notes: event.target.value,
                                                }))
                                            }
                                            rows={2}
                                            className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                        />
                                    </label>
                                </div>
                                <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                                    <button
                                        type="button"
                                        onClick={handleSaveHomeroom}
                                        disabled={savingHomeroom}
                                        className="inline-flex items-center justify-center rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-400"
                                    >
                                        {savingHomeroom ? "保存中..." : "保存班级信息"}
                                    </button>
                                    {homeroomMessage && (
                                        <p className="text-sm text-emerald-600">{homeroomMessage}</p>
                                    )}
                                </div>
                                {homeroomFormError && (
                                    <p className="text-sm text-red-600">错误：{homeroomFormError}</p>
                                )}
                            </section>
                            <section className="space-y-4">
                                <div className="flex items-center justify-between">
                                    <h3 className="text-lg font-semibold text-slate-900">
                                        学生名单（{students.length} 人）
                                    </h3>
                                    <button
                                        type="button"
                                        onClick={handleStartCreateStudent}
                                        className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500"
                                    >
                                        新增学生
                                    </button>
                                </div>
                                {studentsLoading ? (
                                    <p className="text-sm text-slate-500">学生数据加载中...</p>
                                ) : studentsError ? (
                                    <p className="text-sm text-red-600">{studentsError}</p>
                                ) : students.length === 0 ? (
                                    <p className="text-sm text-slate-500">该班级暂无学生，请使用 Excel 导入或手动添加。</p>
                                ) : (
                                    <div className="overflow-x-auto rounded-xl border border-slate-200">
                                        <table className="min-w-full text-sm">
                                            <thead className="bg-slate-50 text-left text-xs uppercase text-slate-500">
                                                <tr>
                                                    <th className="px-4 py-2">姓名</th>
                                                    <th className="px-4 py-2">学生编号</th>
                                                    <th className="px-4 py-2">家长</th>
                                                    <th className="px-4 py-2">教师子女</th>
                                                    <th className="px-4 py-2 text-right">操作</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {students.map((student) => (
                                                    <tr key={student.id} className="border-t">
                                                        <td className="px-4 py-2 font-medium text-slate-900">
                                                            {student.fullName}
                                                        </td>
                                                        <td className="px-4 py-2 text-slate-600">
                                                            {student.studentCode ?? "--"}
                                                        </td>
                                                        <td className="px-4 py-2 text-slate-600">
                                                            {student.primaryGuardianName ?? "--"}
                                                            {student.primaryGuardianPhone
                                                                ? `（${student.primaryGuardianPhone}）`
                                                                : ""}
                                                        </td>
                                                        <td className="px-4 py-2 text-slate-600">
                                                            {student.isTeacherChild ? "是" : "否"}
                                                        </td>
                                                        <td className="px-4 py-2 text-right text-sm text-indigo-600">
                                                            <button
                                                                type="button"
                                                                onClick={() => handleStartEditStudent(student)}
                                                                className="mr-3 hover:underline"
                                                            >
                                                                编辑
                                                            </button>
                                                            <button
                                                                type="button"
                                                                onClick={() => handleDeleteStudent(student.id)}
                                                                className="text-red-600 hover:underline"
                                                            >
                                                                删除
                                                            </button>
                                                        </td>
                                                    </tr>
                                                ))}
                                            </tbody>
                                        </table>
                                    </div>
                                )}
                                {studentFormVisible && (
                                    <div className="space-y-3 rounded-2xl border border-dashed border-slate-300 p-4">
                                        <p className="text-sm font-medium text-slate-900">
                                            {editingStudentId ? "编辑学生" : "新增学生"}
                                        </p>
                                        <div className="grid gap-3 md:grid-cols-2">
                                            <label className="flex flex-col gap-1 text-sm text-slate-700">
                                                姓名
                                                <input
                                                    type="text"
                                                    value={studentForm.fullName}
                                                    onChange={(event) =>
                                                        setStudentForm((prev) => ({
                                                            ...prev,
                                                            fullName: event.target.value,
                                                        }))
                                                    }
                                                    className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                                />
                                            </label>
                                            <label className="flex flex-col gap-1 text-sm text-slate-700">
                                                学生编号
                                                <input
                                                    type="text"
                                                    value={studentForm.studentCode}
                                                    onChange={(event) =>
                                                        setStudentForm((prev) => ({
                                                            ...prev,
                                                            studentCode: event.target.value,
                                                        }))
                                                    }
                                                    className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                                />
                                            </label>
                                            <label className="flex flex-col gap-1 text-sm text-slate-700">
                                                家长姓名
                                                <input
                                                    type="text"
                                                    value={studentForm.primaryGuardianName}
                                                    onChange={(event) =>
                                                        setStudentForm((prev) => ({
                                                            ...prev,
                                                            primaryGuardianName: event.target.value,
                                                        }))
                                                    }
                                                    className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                                />
                                            </label>
                                            <label className="flex flex-col gap-1 text-sm text-slate-700">
                                                家长电话
                                                <input
                                                    type="text"
                                                    value={studentForm.primaryGuardianPhone}
                                                    onChange={(event) =>
                                                        setStudentForm((prev) => ({
                                                            ...prev,
                                                            primaryGuardianPhone: event.target.value,
                                                        }))
                                                    }
                                                    className="rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-900 focus:border-indigo-500 focus:outline-none"
                                                />
                                            </label>
                                            <label className="flex items-center gap-2 text-sm text-slate-700 md:col-span-2">
                                                <input
                                                    type="checkbox"
                                                    checked={studentForm.isTeacherChild}
                                                    onChange={(event) =>
                                                        setStudentForm((prev) => ({
                                                            ...prev,
                                                            isTeacherChild: event.target.checked,
                                                        }))
                                                    }
                                                    className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                                                />
                                                是否教师子女
                                            </label>
                                        </div>
                                        {studentFormError && (
                                            <p className="text-sm text-red-600">{studentFormError}</p>
                                        )}
                                        <div className="flex gap-3">
                                            <button
                                                type="button"
                                                onClick={handleSubmitStudentForm}
                                                disabled={studentSaving}
                                                className="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-400"
                                            >
                                                {studentSaving ? "保存中..." : "保存"}
                                            </button>
                                            <button
                                                type="button"
                                                onClick={handleCancelStudentForm}
                                                className="rounded-lg border border-slate-200 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50"
                                            >
                                                取消
                                            </button>
                                        </div>
                                    </div>
                                )}
                            </section>
                        </Fragment>
                    ) : (
                        <p className="text-sm text-slate-600">请先选择左侧的班级以查看详情。</p>
                    )}
                </main>
            </div>
        </div>
    );
}

function extractError(error: unknown): string {
    if (error instanceof StudentRosterServiceError) {
        return error.message;
    }
    if (error instanceof Error) {
        return error.message;
    }
    return String(error);
}

function createHomeroomForm(): HomeroomFormState {
    return {
        displayName: "",
        headTeacherName: "",
        headTeacherPhone: "",
        notes: "",
    };
}

function createStudentForm(): StudentFormState {
    return {
        fullName: "",
        studentCode: "",
        primaryGuardianName: "",
        primaryGuardianPhone: "",
        isTeacherChild: false,
    };
}

function sortByName(a: RosterStudent, b: RosterStudent) {
    return a.fullName.localeCompare(b.fullName, "zh-CN");
}
