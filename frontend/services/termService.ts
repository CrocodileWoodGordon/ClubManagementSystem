import { ApiClient } from "@/lib/api/client";
import type { Term } from "@/lib/types";

const client = new ApiClient();

export class TermServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "TermServiceError";
    }
}

interface TermApi {
    id: string;
    code: string;
    name: string;
    start_date: string;
    end_date: string;
    enrollment_start: string;
    enrollment_end: string;
    is_active: boolean;
}

function mapTerm(data: TermApi): Term {
    return {
        id: data.id,
        code: data.code,
        name: data.name,
        startDate: data.start_date,
        endDate: data.end_date,
        enrollmentStart: data.enrollment_start,
        enrollmentEnd: data.enrollment_end,
        isActive: data.is_active,
    };
}

export interface CreateTermInput {
    code: string;
    name: string;
    startDate: string;
    endDate: string;
    enrollmentStart: string;
    enrollmentEnd: string;
    isActive: boolean;
}

export interface UpdateTermInput {
    code?: string;
    name?: string;
    startDate?: string;
    endDate?: string;
    enrollmentStart?: string;
    enrollmentEnd?: string;
    isActive?: boolean;
}

function mapCreateInput(input: CreateTermInput) {
    return {
        code: input.code,
        name: input.name,
        start_date: input.startDate,
        end_date: input.endDate,
        enrollment_start: input.enrollmentStart,
        enrollment_end: input.enrollmentEnd,
        is_active: input.isActive,
    };
}

function mapUpdateInput(input: UpdateTermInput) {
    const payload: Record<string, unknown> = {};
    if (input.code !== undefined) {
        payload.code = input.code;
    }
    if (input.name !== undefined) {
        payload.name = input.name;
    }
    if (input.startDate !== undefined) {
        payload.start_date = input.startDate;
    }
    if (input.endDate !== undefined) {
        payload.end_date = input.endDate;
    }
    if (input.enrollmentStart !== undefined) {
        payload.enrollment_start = input.enrollmentStart;
    }
    if (input.enrollmentEnd !== undefined) {
        payload.enrollment_end = input.enrollmentEnd;
    }
    if (typeof input.isActive === "boolean") {
        payload.is_active = input.isActive;
    }
    return payload;
}

export async function fetchTerms(): Promise<Term[]> {
    try {
        const response = await client.get<TermApi[]>("/api/admin/terms");
        return response.map(mapTerm);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new TermServiceError(`获取学期列表失败: ${message}`);
    }
}

export async function createTerm(input: CreateTermInput): Promise<Term> {
    try {
        const response = await client.post<TermApi>("/api/admin/terms", mapCreateInput(input));
        return mapTerm(response);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new TermServiceError(`创建学期失败: ${message}`);
    }
}

export async function updateTerm(termId: string, input: UpdateTermInput): Promise<Term> {
    try {
        const response = await client.put<TermApi>(`/api/admin/terms/${termId}`, mapUpdateInput(input));
        return mapTerm(response);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new TermServiceError(`更新学期失败: ${message}`);
    }
}

export async function deleteTerm(termId: string): Promise<void> {
    try {
        await client.delete(`/api/admin/terms/${termId}`);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new TermServiceError(`删除学期失败: ${message}`);
    }
}

export async function activateTerm(termId: string): Promise<Term> {
    try {
        const response = await client.post<TermApi>(`/api/admin/terms/${termId}/activate`, {});
        return mapTerm(response);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new TermServiceError(`切换当前学期失败: ${message}`);
    }
}
