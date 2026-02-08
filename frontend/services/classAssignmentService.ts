import { ApiClient } from "@/lib/api/client";
import type { ClassInstance } from "@/lib/types";

const client = new ApiClient();

export class ClassAssignmentServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "ClassAssignmentServiceError";
    }
}

export interface ClassQueryParams {
    termId?: string;
    campusId: string;
    clubId: string;
    weekday: number;
}

export interface CreateClassPayload {
    termId?: string;
    campusId: string;
    clubId: string;
    weekday: number;
    classCode: string;
    startTime: string;
    endTime: string;
    location?: string;
    capacity?: number;
    notes?: string;
}

export interface AssignStudentsPayload {
    termId?: string;
    campusId: string;
    clubId: string;
    weekday: number;
    classId?: string | null;
    enrollmentIds: string[];
}

export interface UpdateClassPayload extends CreateClassPayload {
    id: string;
}

export async function fetchClassesForSlot(
    params: ClassQueryParams,
): Promise<ClassInstance[]> {
    return safeRequest("获取班级配置失败", async () => {
        const query = buildQueryString({
            term_id: params.termId,
            campus_id: params.campusId,
            club_id: params.clubId,
            weekday: String(params.weekday),
        });
        const res = await client.get<ClassListResponse>(`/api/classes${query}`);
        return res.data.map(mapClassInstance);
    });
}

export async function createClass(payload: CreateClassPayload): Promise<ClassInstance> {
    return safeRequest("创建班级失败", async () => {
        const res = await client.post<ClassDetailResponse>("/api/classes", {
            term_id: payload.termId,
            campus_id: payload.campusId,
            club_id: payload.clubId,
            weekday: payload.weekday,
            class_code: payload.classCode,
            start_time: payload.startTime,
            end_time: payload.endTime,
            location: payload.location,
            capacity: payload.capacity,
            notes: payload.notes,
        });
        return mapClassInstance(res.data);
    });
}

export async function assignStudentsToClass(payload: AssignStudentsPayload): Promise<number> {
    if (payload.enrollmentIds.length === 0) {
        throw new ClassAssignmentServiceError("请选择至少一名学生再进行分班操作");
    }
    return safeRequest("分配班级失败", async () => {
        const res = await client.post<AssignmentResponse>("/api/classes/assign", {
            term_id: payload.termId,
            campus_id: payload.campusId,
            club_id: payload.clubId,
            weekday: payload.weekday,
            class_id: payload.classId ?? null,
            enrollment_ids: payload.enrollmentIds,
        });
        return res.updated;
    });
}

export async function updateClass(payload: UpdateClassPayload): Promise<ClassInstance> {
    return safeRequest("更新班级失败", async () => {
        const res = await client.put<ClassDetailResponse>(`/api/classes/${payload.id}`, {
            term_id: payload.termId,
            campus_id: payload.campusId,
            club_id: payload.clubId,
            weekday: payload.weekday,
            class_code: payload.classCode,
            start_time: payload.startTime,
            end_time: payload.endTime,
            location: payload.location,
            capacity: payload.capacity,
            notes: payload.notes,
        });
        return mapClassInstance(res.data);
    });
}

async function safeRequest<T>(
    context: string,
    executor: () => Promise<T>,
): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof ClassAssignmentServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new ClassAssignmentServiceError(`${context}: ${message}`);
    }
}

function buildQueryString(values: Record<string, string | undefined>): string {
    const params = new URLSearchParams();
    Object.entries(values).forEach(([key, value]) => {
        if (value && value.length > 0) {
            params.set(key, value);
        }
    });
    const query = params.toString();
    return query ? `?${query}` : "";
}

function mapClassInstance(payload: ClassApiResponse): ClassInstance {
    return {
        id: payload.id,
        termId: payload.term_id,
        campusId: payload.campus_id,
        clubId: payload.club_id,
        classCode: payload.class_code,
        weekday: payload.weekday,
        startTime: payload.start_time,
        endTime: payload.end_time,
        location: payload.location ?? undefined,
        capacity: payload.capacity ?? undefined,
        status: payload.status,
        notes: payload.notes ?? undefined,
        assignedCount: payload.assigned_count,
    };
}

interface ClassListResponse {
    data: ClassApiResponse[];
}

interface ClassDetailResponse {
    data: ClassApiResponse;
}

interface ClassApiResponse {
    id: string;
    term_id: string;
    campus_id: string;
    club_id: string;
    class_code: string;
    weekday: number;
    start_time: string;
    end_time: string;
    location: string | null;
    capacity: number | null;
    status: ClassInstance["status"];
    notes: string | null;
    assigned_count: number;
}

interface AssignmentResponse {
    updated: number;
}
