import { ApiClient } from "@/lib/api/client";
import type { Club, ClubMember } from "@/lib/types";

const client = new ApiClient();

export class ClubServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "ClubServiceError";
    }
}

export interface ClubListParams {
    search?: string;
}

export interface UpsertClubInput {
    code: string;
    name: string;
    description?: string;
    materialFee?: number;
    pricePerSession?: number;
    graceSessions?: number;
}

export interface ClubMemberParams {
    clubId: string;
    termId: string;
    campusId: string;
    weekday?: number;
}

export interface AddClubMembersInput {
    termId: string;
    campusId: string;
    entries: Array<{
        studentId: string;
        requestedWeekday: number;
    }>;
}

export async function fetchClubs(params?: ClubListParams): Promise<Club[]> {
    return safeRequest("获取社团列表失败", async () => {
        const query = buildQueryString({ search: params?.search });
        const response = await client.get<ClubListApiResponse>(`/api/clubs${query}`);
        return response.data.map(mapClub);
    });
}

export async function createClub(payload: UpsertClubInput): Promise<Club> {
    return safeRequest("创建社团失败", async () => {
        const response = await client.post<ClubDetailApiResponse>("/api/clubs", {
            code: payload.code,
            name: payload.name,
            description: sanitize(payload.description),
            material_fee: payload.materialFee ?? 0,
            price_per_session: payload.pricePerSession ?? 0,
            grace_sessions: payload.graceSessions ?? 3,
        });
        return mapClub(response.data);
    });
}

export async function updateClub(
    clubId: string,
    payload: UpsertClubInput,
): Promise<Club> {
    return safeRequest("更新社团失败", async () => {
        const response = await client.put<ClubDetailApiResponse>(`/api/clubs/${clubId}`, {
            code: sanitize(payload.code),
            name: sanitize(payload.name),
            description: sanitize(payload.description),
            material_fee: payload.materialFee,
            price_per_session: payload.pricePerSession,
            grace_sessions: payload.graceSessions,
        });
        return mapClub(response.data);
    });
}

export async function deleteClub(clubId: string): Promise<void> {
    return safeRequest("删除社团失败", async () => {
        await client.delete(`/api/clubs/${clubId}`);
    });
}

export async function fetchClubMembers(
    params: ClubMemberParams,
): Promise<ClubMember[]> {
    if (!params.termId || !params.campusId) {
        throw new ClubServiceError("请先选择学期和校区");
    }
    if (params.weekday !== undefined && (params.weekday < 1 || params.weekday > 7)) {
        throw new ClubServiceError("星期需在 1-7 之间（1 表示周一）");
    }
    return safeRequest("获取社团成员失败", async () => {
        const query = buildQueryString({
            term_id: params.termId,
            campus_id: params.campusId,
            weekday:
                params.weekday !== undefined ? String(params.weekday) : undefined,
        });
        const response = await client.get<ClubMemberListApiResponse>(
            `/api/clubs/${params.clubId}/members${query}`,
        );
        return response.data.map(mapClubMember);
    });
}

export async function addClubMembers(
    clubId: string,
    payload: AddClubMembersInput,
): Promise<ClubMember[]> {
    return safeRequest("添加社团成员失败", async () => {
        const response = await client.post<ClubMemberListApiResponse>(
            `/api/clubs/${clubId}/members`,
            {
                term_id: payload.termId,
                campus_id: payload.campusId,
                entries: payload.entries.map((entry) => ({
                    student_id: entry.studentId,
                    requested_weekday: entry.requestedWeekday,
                })),
            },
        );
        return response.data.map(mapClubMember);
    });
}

export async function removeClubMember(
    clubId: string,
    enrollmentId: string,
): Promise<void> {
    return safeRequest("移除社团成员失败", async () => {
        await client.delete(`/api/clubs/${clubId}/members/${enrollmentId}`);
    });
}

function sanitize(value?: string): string | undefined {
    if (!value) {
        return undefined;
    }
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
}

function mapClub(data: ClubApi): Club {
    return {
        id: data.id,
        code: data.code,
        name: data.name,
        description: data.description ?? undefined,
        materialFee: Number(data.material_fee),
        pricePerSession: Number(data.price_per_session),
        graceSessions: data.grace_sessions,
        createdAt: data.created_at,
    };
}

function mapClubMember(data: ClubMemberApi): ClubMember {
    return {
        enrollmentId: data.enrollment_id,
        studentId: data.student_id,
        studentName: data.student_name,
        studentCode: data.student_code ?? undefined,
        homeroom: data.homeroom,
        campusId: data.campus_id,
        campusName: data.campus_name,
        termId: data.term_id,
        requestedWeekday: data.requested_weekday,
        status: data.status,
    };
}

async function safeRequest<T>(
    context: string,
    executor: () => Promise<T>,
): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof ClubServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new ClubServiceError(`${context}: ${message}`);
    }
}

function buildQueryString(values: Record<string, string | undefined>): string {
    const search = new URLSearchParams();
    Object.entries(values).forEach(([key, value]) => {
        if (value) {
            search.set(key, value);
        }
    });
    const query = search.toString();
    return query ? `?${query}` : "";
}

interface ClubListApiResponse {
    data: ClubApi[];
}

interface ClubDetailApiResponse {
    data: ClubApi;
}

interface ClubMemberListApiResponse {
    data: ClubMemberApi[];
}

interface ClubApi {
    id: string;
    code: string;
    name: string;
    description: string | null;
    material_fee: number;
    price_per_session: number;
    grace_sessions: number;
    created_at: string;
}

interface ClubMemberApi {
    enrollment_id: string;
    student_id: string;
    student_name: string;
    student_code: string | null;
    homeroom: string;
    campus_id: string;
    campus_name: string;
    term_id: string;
    requested_weekday: number;
    status: ClubMember["status"];
}
