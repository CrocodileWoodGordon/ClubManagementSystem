"use client";

import { useCallback, useEffect, useMemo, useState } from "react";

import { SectionCard } from "@/components/common/SectionCard";
import type { Club, ClubMember, HomeroomRoster, RosterStudent } from "@/lib/types";
import {
    ClubServiceError,
    addClubMembers,
    createClub,
    deleteClub,
    fetchClubMembers,
    fetchClubs,
    removeClubMember,
    updateClub,
} from "@/services/clubService";
import {
    fetchHomeroomStudents,
    fetchHomerooms,
} from "@/services/studentRosterService";

interface TermOption {
    id: string;
    code: string;
    name: string;
    isActive: boolean;
}

interface CampusOption {
    id: string;
    name: string;
    shortName?: string;
}

interface ClubManagementWorkspaceProps {
    terms: TermOption[];
    campuses: CampusOption[];
    initialClubs: Club[];
    defaultTermId?: string;
    initialError?: string;
}

interface ClubFormState {
    code: string;
    name: string;
    description: string;
    materialFee: string;
    pricePerSession: string;
    graceSessions: string;
}

const WEEKDAY_OPTIONS = [
    { value: 1, label: "周一" },
    { value: 2, label: "周二" },
    { value: 3, label: "周三" },
    { value: 4, label: "周四" },
    { value: 5, label: "周五" },
    { value: 6, label: "周六" },
    { value: 7, label: "周日" },
];

export function ClubManagementWorkspace({
    terms,
    campuses,
    initialClubs,
    defaultTermId,
    initialError,
}: ClubManagementWorkspaceProps) {
    const [clubs, setClubs] = useState<Club[]>(initialClubs);
    const [search, setSearch] = useState<string>("");
    const [clubError, setClubError] = useState<string | null>(initialError ?? null);
    const [isRefreshing, setIsRefreshing] = useState(false);
    const [selectedClubId, setSelectedClubId] = useState<string | null>(
        initialClubs[0]?.id ?? null,
    );

    const [editForm, setEditForm] = useState<ClubFormState>(
        buildClubFormState(initialClubs[0]),
    );
    const [createForm, setCreateForm] = useState<ClubFormState>({
        code: "",
        name: "",
        description: "",
        materialFee: "0",
        pricePerSession: "0",
        graceSessions: "3",
    });
    const [isSavingClub, setIsSavingClub] = useState(false);

    const [filterTermId, setFilterTermId] = useState<string>(
        defaultTermId ?? terms[0]?.id ?? "",
    );
    const [filterCampusId, setFilterCampusId] = useState<string>(
        campuses[0]?.id ?? "",
    );
    const [weekdayFilter, setWeekdayFilter] = useState<string>("");
    const [members, setMembers] = useState<ClubMember[]>([]);
    const [memberError, setMemberError] = useState<string | null>(null);
    const [isLoadingMembers, setIsLoadingMembers] = useState(false);

    const [homerooms, setHomerooms] = useState<HomeroomRoster[]>([]);
    const [isLoadingHomerooms, setIsLoadingHomerooms] = useState(false);
    const [homeroomError, setHomeroomError] = useState<string | null>(null);
    const [selectedHomeroomId, setSelectedHomeroomId] = useState<string>("");
    const [studentsByHomeroom, setStudentsByHomeroom] = useState<
        Record<string, RosterStudent[]>
    >({});
    const [isLoadingStudents, setIsLoadingStudents] = useState(false);
    const [studentError, setStudentError] = useState<string | null>(null);
    const [selectedStudentId, setSelectedStudentId] = useState<string>("");
    const [newMemberWeekday, setNewMemberWeekday] = useState<number>(1);
    const [memberActionMessage, setMemberActionMessage] = useState<string | null>(null);

    const selectedClub = useMemo(
        () => clubs.find((club) => club.id === selectedClubId) ?? null,
        [clubs, selectedClubId],
    );

    useEffect(() => {
        setEditForm(buildClubFormState(selectedClub ?? undefined));
    }, [selectedClub]);

    const refreshClubs = useCallback(
        async (targetId?: string) => {
            setIsRefreshing(true);
            setClubError(null);
            try {
                const keyword = search.trim();
                const data = await fetchClubs(
                    keyword.length > 0 ? { search: keyword } : undefined,
                );
                setClubs(data);
                if (data.length === 0) {
                    setSelectedClubId(null);
                } else if (targetId) {
                    setSelectedClubId(targetId);
                } else if (!data.find((club) => club.id === selectedClubId)) {
                    setSelectedClubId(data[0].id);
                }
            } catch (error) {
                setClubError(extractError(error));
            } finally {
                setIsRefreshing(false);
            }
        },
        [search, selectedClubId],
    );

    const loadMembers = useCallback(async () => {
        if (!selectedClubId || !filterTermId || !filterCampusId) {
            setMembers([]);
            return;
        }
        setIsLoadingMembers(true);
        setMemberError(null);
        try {
            const membersData = await fetchClubMembers({
                clubId: selectedClubId,
                termId: filterTermId,
                campusId: filterCampusId,
                weekday: weekdayFilter ? Number(weekdayFilter) : undefined,
            });
            setMembers(membersData);
        } catch (error) {
            setMemberError(extractError(error));
        } finally {
            setIsLoadingMembers(false);
        }
    }, [selectedClubId, filterTermId, filterCampusId, weekdayFilter]);

    useEffect(() => {
        void loadMembers();
    }, [loadMembers]);

    const loadHomerooms = useCallback(async () => {
        if (!filterTermId || !filterCampusId) {
            setHomerooms([]);
            setSelectedHomeroomId("");
            return;
        }
        setIsLoadingHomerooms(true);
        setHomeroomError(null);
        try {
            const data = await fetchHomerooms({
                termId: filterTermId,
                campusId: filterCampusId,
            });
            setHomerooms(data);
            if (!data.find((item) => item.id === selectedHomeroomId)) {
                setSelectedHomeroomId("");
                setSelectedStudentId("");
            }
        } catch (error) {
            setHomeroomError(extractError(error));
        } finally {
            setIsLoadingHomerooms(false);
        }
    }, [filterTermId, filterCampusId, selectedHomeroomId]);

    useEffect(() => {
        void loadHomerooms();
    }, [loadHomerooms]);

    useEffect(() => {
        if (!selectedHomeroomId || studentsByHomeroom[selectedHomeroomId]) {
            return;
        }
        setIsLoadingStudents(true);
        setStudentError(null);
        fetchHomeroomStudents(selectedHomeroomId, { termId: filterTermId })
            .then((students) => {
                setStudentsByHomeroom((prev) => ({
                    ...prev,
                    [selectedHomeroomId]: students,
                }));
            })
            .catch((error) => {
                setStudentError(extractError(error));
            })
            .finally(() => setIsLoadingStudents(false));
    }, [selectedHomeroomId, filterTermId, studentsByHomeroom]);

    useEffect(() => {
        setSelectedStudentId("");
    }, [selectedHomeroomId]);

    const handleCreateClub = async () => {
        setIsSavingClub(true);
        setClubError(null);
        try {
            const payload = formStateToPayload(createForm);
            const club = await createClub(payload);
            await refreshClubs(club.id);
            setCreateForm({
                code: "",
                name: "",
                description: "",
                materialFee: "0",
                pricePerSession: "0",
                graceSessions: "3",
            });
        } catch (error) {
            setClubError(extractError(error));
        } finally {
            setIsSavingClub(false);
        }
    };

    const handleUpdateClub = async () => {
        if (!selectedClubId) {
            return;
        }
        setIsSavingClub(true);
        setClubError(null);
        try {
            const payload = formStateToPayload(editForm);
            await updateClub(selectedClubId, payload);
            await refreshClubs(selectedClubId);
        } catch (error) {
            setClubError(extractError(error));
        } finally {
            setIsSavingClub(false);
        }
    };

    const handleDeleteClub = async () => {
        if (!selectedClubId || !selectedClub) {
            return;
        }
        const warning =
            `删除“${selectedClub.name}”将同步清空所有相关报名/学生数据，操作不可撤销。\n` +
            "确认继续？";
        if (!window.confirm(warning)) {
            return;
        }
        const doubleConfirm = window.confirm("再次确认，确定要删除该社团及其报名数据吗？");
        if (!doubleConfirm) {
            return;
        }
        setIsSavingClub(true);
        setClubError(null);
        try {
            await deleteClub(selectedClubId);
            await refreshClubs();
        } catch (error) {
            setClubError(extractError(error));
        } finally {
            setIsSavingClub(false);
        }
    };

    const handleAddMember = async () => {
        if (
            !selectedClubId ||
            !filterTermId ||
            !filterCampusId ||
            !selectedStudentId
        ) {
            setMemberActionMessage("请先选定学期、校区、班级与学生");
            return;
        }
        setMemberActionMessage(null);
        try {
            await addClubMembers(selectedClubId, {
                termId: filterTermId,
                campusId: filterCampusId,
                entries: [
                    {
                        studentId: selectedStudentId,
                        requestedWeekday: newMemberWeekday,
                    },
                ],
            });
            setMemberActionMessage("已添加新成员");
            setSelectedStudentId("");
            await loadMembers();
        } catch (error) {
            setMemberActionMessage(extractError(error));
        }
    };

    const handleRemoveMember = async (enrollmentId: string) => {
        if (!selectedClubId) {
            return;
        }
        if (
            !window.confirm("移除后将把该学生从社团报名中删除，并标记为退课，确认吗？")
        ) {
            return;
        }
        try {
            await removeClubMember(selectedClubId, enrollmentId);
            await loadMembers();
        } catch (error) {
            setMemberActionMessage(extractError(error));
        }
    };

    return (
        <div className="space-y-8">
            <SectionCard
                title="社团列表"
                description="集中查看社团，选中后可在下方编辑详细信息。"
            >
                <div className="flex flex-wrap gap-3">
                    <input
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        placeholder="按名称/编码搜索"
                        className="flex-1 min-w-[180px] rounded-lg border border-slate-300 px-3 py-2 text-sm"
                    />
                    <button
                        type="button"
                        onClick={() => refreshClubs()}
                        className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow hover:bg-indigo-500 disabled:opacity-50"
                        disabled={isRefreshing}
                    >
                        {isRefreshing ? "加载中..." : "查询"}
                    </button>
                </div>
                {clubError ? (
                    <p className="mt-3 text-sm text-rose-600">{clubError}</p>
                ) : null}
                <div className="mt-4 grid gap-3 md:grid-cols-2 lg:grid-cols-3">
                    {clubs.map((club) => (
                        <button
                            key={club.id}
                            type="button"
                            onClick={() => setSelectedClubId(club.id)}
                            className={[
                                "rounded-xl border px-4 py-3 text-left transition",
                                club.id === selectedClubId
                                    ? "border-indigo-500 bg-indigo-50 shadow"
                                    : "border-slate-200 hover:border-indigo-200",
                            ].join(" ")}
                        >
                            <p className="text-sm font-semibold text-slate-900">
                                {club.name}
                            </p>
                            <p className="text-xs text-slate-500">{club.code}</p>
                            <p className="text-xs text-slate-500">
                                材料费 ¥{club.materialFee.toFixed(2)} / 课时费 ¥
                                {club.pricePerSession.toFixed(2)}
                            </p>
                        </button>
                    ))}
                    {clubs.length === 0 ? (
                        <p className="text-sm text-slate-500">
                            暂无社团，请先创建。
                        </p>
                    ) : null}
                </div>
            </SectionCard>

            <SectionCard
                title="编辑社团"
                description="修改名称/费用或删除社团，删除操作会连同报名记录一起移除。"
            >
                {selectedClub ? (
                    <div className="space-y-4">
                        <div className="grid gap-4 md:grid-cols-2">
                            <TextField
                                label="编码"
                                value={editForm.code}
                                onChange={(value) =>
                                    setEditForm((prev) => ({ ...prev, code: value }))
                                }
                            />
                            <TextField
                                label="名称"
                                value={editForm.name}
                                onChange={(value) =>
                                    setEditForm((prev) => ({ ...prev, name: value }))
                                }
                            />
                            <TextField
                                label="材料费 (¥)"
                                type="number"
                                value={editForm.materialFee}
                                onChange={(value) =>
                                    setEditForm((prev) => ({
                                        ...prev,
                                        materialFee: value,
                                    }))
                                }
                            />
                            <TextField
                                label="课时费 (¥)"
                                type="number"
                                value={editForm.pricePerSession}
                                onChange={(value) =>
                                    setEditForm((prev) => ({
                                        ...prev,
                                        pricePerSession: value,
                                    }))
                                }
                            />
                            <TextField
                                label="免课节数"
                                type="number"
                                value={editForm.graceSessions}
                                onChange={(value) =>
                                    setEditForm((prev) => ({
                                        ...prev,
                                        graceSessions: value,
                                    }))
                                }
                            />
                            <div className="md:col-span-2">
                                <label className="text-sm font-medium text-slate-700">
                                    说明
                                </label>
                                <textarea
                                    value={editForm.description}
                                    onChange={(event) =>
                                        setEditForm((prev) => ({
                                            ...prev,
                                            description: event.target.value,
                                        }))
                                    }
                                    rows={3}
                                    className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
                                />
                            </div>
                        </div>
                        <div className="flex flex-wrap gap-3">
                            <button
                                type="button"
                                onClick={handleUpdateClub}
                                disabled={isSavingClub}
                                className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow hover:bg-indigo-500 disabled:opacity-50"
                            >
                                保存修改
                            </button>
                            <button
                                type="button"
                                onClick={handleDeleteClub}
                                disabled={isSavingClub}
                                className="rounded-lg bg-white px-4 py-2 text-sm font-medium text-rose-600 ring-1 ring-rose-200 hover:bg-rose-50 disabled:opacity-50"
                            >
                                删除社团
                            </button>
                        </div>
                    </div>
                ) : (
                    <p className="text-sm text-slate-500">暂无可编辑的社团。</p>
                )}
            </SectionCard>

            <SectionCard
                title="新增社团"
                description="补充新的社团定义，创建后可立即用于报名与分班。"
            >
                <div className="grid gap-4 md:grid-cols-2">
                    <TextField
                        label="编码"
                        value={createForm.code}
                        onChange={(value) =>
                            setCreateForm((prev) => ({ ...prev, code: value }))
                        }
                    />
                    <TextField
                        label="名称"
                        value={createForm.name}
                        onChange={(value) =>
                            setCreateForm((prev) => ({ ...prev, name: value }))
                        }
                    />
                    <TextField
                        label="材料费 (¥)"
                        type="number"
                        value={createForm.materialFee}
                        onChange={(value) =>
                            setCreateForm((prev) => ({ ...prev, materialFee: value }))
                        }
                    />
                    <TextField
                        label="课时费 (¥)"
                        type="number"
                        value={createForm.pricePerSession}
                        onChange={(value) =>
                            setCreateForm((prev) => ({
                                ...prev,
                                pricePerSession: value,
                            }))
                        }
                    />
                    <TextField
                        label="免课节数"
                        type="number"
                        value={createForm.graceSessions}
                        onChange={(value) =>
                            setCreateForm((prev) => ({
                                ...prev,
                                graceSessions: value,
                            }))
                        }
                    />
                    <div className="md:col-span-2">
                        <label className="text-sm font-medium text-slate-700">
                            说明
                        </label>
                        <textarea
                            value={createForm.description}
                            onChange={(event) =>
                                setCreateForm((prev) => ({
                                    ...prev,
                                    description: event.target.value,
                                }))
                            }
                            rows={3}
                            className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
                        />
                    </div>
                </div>
                <button
                    type="button"
                    onClick={handleCreateClub}
                    disabled={isSavingClub}
                    className="mt-4 rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white shadow hover:bg-slate-800 disabled:opacity-50"
                >
                    创建社团
                </button>
            </SectionCard>

            <SectionCard
                title="社团成员管理"
                description="按照学期/校区筛选，增删社团报名成员，支持即时同步数据。"
            >
                {selectedClub ? (
                    <div className="space-y-5">
                        <div className="grid gap-3 md:grid-cols-3">
                            <SelectField
                                label="学期"
                                value={filterTermId}
                                onChange={(value) => setFilterTermId(value)}
                                options={terms.map((term) => ({
                                    value: term.id,
                                    label: `${term.code} ${term.name}${
                                        term.isActive ? "（当前）" : ""
                                    }`,
                                }))}
                            />
                            <SelectField
                                label="校区"
                                value={filterCampusId}
                                onChange={(value) => setFilterCampusId(value)}
                                options={campuses.map((campus) => ({
                                    value: campus.id,
                                    label: campus.shortName
                                        ? `${campus.name}（${campus.shortName}）`
                                        : campus.name,
                                }))}
                            />
                            <SelectField
                                label="星期"
                                value={weekdayFilter}
                                onChange={(value) => setWeekdayFilter(value)}
                                options={[
                                    { value: "", label: "全部" },
                                    ...WEEKDAY_OPTIONS.map((item) => ({
                                        value: String(item.value),
                                        label: item.label,
                                    })),
                                ]}
                            />
                        </div>
                        <div className="flex items-center justify-between">
                            <p className="text-sm text-slate-500">
                                当前社团：{selectedClub.name}
                            </p>
                            <button
                                type="button"
                                onClick={() => loadMembers()}
                                className="text-sm text-indigo-600 hover:underline"
                            >
                                重新加载
                            </button>
                        </div>
                        {memberError ? (
                            <p className="text-sm text-rose-600">{memberError}</p>
                        ) : null}
                        <div className="overflow-x-auto rounded-xl border border-slate-200">
                            <table className="min-w-full divide-y divide-slate-200 text-sm">
                                <thead className="bg-slate-50">
                                    <tr>
                                        <th className="px-4 py-2 text-left font-medium text-slate-600">
                                            学生
                                        </th>
                                        <th className="px-4 py-2 text-left font-medium text-slate-600">
                                            班级
                                        </th>
                                        <th className="px-4 py-2 text-left font-medium text-slate-600">
                                            星期
                                        </th>
                                        <th className="px-4 py-2 text-left font-medium text-slate-600">
                                            状态
                                        </th>
                                        <th className="px-4 py-2" />
                                    </tr>
                                </thead>
                                <tbody className="divide-y divide-slate-100 bg-white">
                                    {isLoadingMembers ? (
                                        <tr>
                                            <td
                                                colSpan={5}
                                                className="px-4 py-3 text-center text-slate-500"
                                            >
                                                正在加载成员...
                                            </td>
                                        </tr>
                                    ) : members.length === 0 ? (
                                        <tr>
                                            <td
                                                colSpan={5}
                                                className="px-4 py-3 text-center text-slate-500"
                                            >
                                                暂无成员，添加后会显示。
                                            </td>
                                        </tr>
                                    ) : (
                                        members.map((member) => (
                                            <tr key={member.enrollmentId}>
                                                <td className="px-4 py-2 text-slate-900">
                                                    {member.studentName}
                                                </td>
                                                <td className="px-4 py-2 text-slate-600">
                                                    {member.homeroom}
                                                </td>
                                                <td className="px-4 py-2 text-slate-600">
                                                    {formatWeekday(member.requestedWeekday)}
                                                </td>
                                                <td className="px-4 py-2 text-slate-600">
                                                    {member.status}
                                                </td>
                                                <td className="px-4 py-2 text-right">
                                                    <button
                                                        type="button"
                                                        onClick={() =>
                                                            handleRemoveMember(member.enrollmentId)
                                                        }
                                                        className="text-sm text-rose-600 hover:underline"
                                                    >
                                                        移除
                                                    </button>
                                                </td>
                                            </tr>
                                        ))
                                    )}
                                </tbody>
                            </table>
                        </div>
                        <div className="rounded-2xl border border-dashed border-slate-300 p-4">
                            <p className="text-sm font-semibold text-slate-700">
                                手动添加成员
                            </p>
                            <div className="mt-3 grid gap-3 md:grid-cols-3">
                                <SelectField
                                    label="班级"
                                    value={selectedHomeroomId}
                                    onChange={(value) => setSelectedHomeroomId(value)}
                                    options={[
                                        { value: "", label: "选择班级" },
                                        ...homerooms.map((homeroom) => ({
                                            value: homeroom.id,
                                            label: homeroom.displayName,
                                        })),
                                    ]}
                                />
                                <SelectField
                                    label="学生"
                                    value={selectedStudentId}
                                    onChange={(value) => setSelectedStudentId(value)}
                                    options={[
                                        { value: "", label: "选择学生" },
                                        ...(selectedHomeroomId
                                            ? (studentsByHomeroom[selectedHomeroomId] ?? []).map(
                                                  (student) => ({
                                                      value: student.id,
                                                      label: student.fullName,
                                                  }),
                                              )
                                            : []),
                                    ]}
                                    disabled={!selectedHomeroomId || isLoadingStudents}
                                />
                                <SelectField
                                    label="星期"
                                    value={String(newMemberWeekday)}
                                    onChange={(value) =>
                                        setNewMemberWeekday(Number(value) || 1)
                                    }
                                    options={WEEKDAY_OPTIONS.map((item) => ({
                                        value: String(item.value),
                                        label: item.label,
                                    }))}
                                />
                            </div>
                            {isLoadingHomerooms ? (
                                <p className="mt-2 text-sm text-slate-500">正在加载班级...</p>
                            ) : null}
                            {homeroomError ? (
                                <p className="mt-2 text-sm text-rose-600">{homeroomError}</p>
                            ) : null}
                            {studentError ? (
                                <p className="mt-2 text-sm text-rose-600">{studentError}</p>
                            ) : null}
                            {memberActionMessage ? (
                                <p className="mt-2 text-sm text-slate-600">
                                    {memberActionMessage}
                                </p>
                            ) : null}
                            <button
                                type="button"
                                onClick={handleAddMember}
                                className="mt-3 rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow hover:bg-emerald-500"
                            >
                                添加
                            </button>
                        </div>
                    </div>
                ) : (
                    <p className="text-sm text-slate-500">
                        请先选择社团后再管理成员。
                    </p>
                )}
            </SectionCard>
        </div>
    );
}

function buildClubFormState(club?: Club | null): ClubFormState {
    if (!club) {
        return {
            code: "",
            name: "",
            description: "",
            materialFee: "0",
            pricePerSession: "0",
            graceSessions: "3",
        };
    }
    return {
        code: club.code,
        name: club.name,
        description: club.description ?? "",
        materialFee: String(club.materialFee),
        pricePerSession: String(club.pricePerSession),
        graceSessions: String(club.graceSessions),
    };
}

function formStateToPayload(form: ClubFormState) {
    return {
        code: form.code,
        name: form.name,
        description: form.description,
        materialFee: Number(form.materialFee),
        pricePerSession: Number(form.pricePerSession),
        graceSessions: Number(form.graceSessions),
    };
}

function extractError(error: unknown): string {
    if (error instanceof ClubServiceError) {
        return error.message;
    }
    if (error instanceof Error) {
        return error.message;
    }
    return String(error);
}

interface TextFieldProps {
    label: string;
    value: string;
    onChange: (value: string) => void;
    type?: "text" | "number";
}

function TextField({ label, value, onChange, type = "text" }: TextFieldProps) {
    return (
        <label className="text-sm font-medium text-slate-700">
            {label}
            <input
                type={type}
                value={value}
                onChange={(event) => onChange(event.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm"
            />
        </label>
    );
}

interface SelectFieldProps {
    label: string;
    value: string;
    onChange: (value: string) => void;
    options: { value: string; label: string }[];
    disabled?: boolean;
}

function SelectField({
    label,
    value,
    onChange,
    options,
    disabled,
}: SelectFieldProps) {
    return (
        <label className="text-sm font-medium text-slate-700">
            {label}
            <select
                value={value}
                onChange={(event) => onChange(event.target.value)}
                disabled={disabled}
                className="mt-1 w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm disabled:bg-slate-100"
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

function formatWeekday(value: number): string {
    const match = WEEKDAY_OPTIONS.find((item) => item.value === value);
    return match ? match.label : `周${value}`;
}
