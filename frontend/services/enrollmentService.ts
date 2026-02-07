import { ApiClient } from "@/lib/api/client";
import type {
    EnrollmentImportConfig,
    EnrollmentImportOutcome,
    EnrollmentSummaryRow,
    PendingEnrollment,
} from "@/lib/types";

const client = new ApiClient();
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export class EnrollmentServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "EnrollmentServiceError";
    }
}

export interface EnrollmentListParams {
    termId?: string;
    campusId?: string;
    homeroom?: string;
    club?: string;
    weekday?: number;
    studentName?: string;
}

export interface EnrollmentImportOptions {
    file: File;
    config?: EnrollmentImportConfig;
}

export async function fetchPendingEnrollments(
    params?: EnrollmentListParams,
): Promise<PendingEnrollment[]> {
    return safeRequest("获取待分班名单失败", async () => {
        const search = buildQueryString({
            term_id: params?.termId,
            campus_id: params?.campusId,
            homeroom: params?.homeroom,
            club: params?.club,
            weekday: params?.weekday !== undefined ? String(params.weekday) : undefined,
            student_name: params?.studentName,
        });
        const response = await client.get<EnrollmentListResponse>(
            `/api/enrollments/pending${search}`,
        );
        return response.data.map(mapPendingEnrollment);
    });
}

export interface EnrollmentSummaryParams {
    termId?: string;
    campusId?: string;
}

export interface EnrollmentSlotParams {
    termId?: string;
    campusId: string;
    clubId: string;
    weekday: number;
}

export async function fetchEnrollmentSummary(
    params?: EnrollmentSummaryParams,
): Promise<EnrollmentSummaryRow[]> {
    return safeRequest("获取报名汇总失败", async () => {
        const search = buildQueryString({
            term_id: params?.termId,
            campus_id: params?.campusId,
        });
        const response = await client.get<EnrollmentSummaryResponse>(
            `/api/enrollments/summary${search}`,
        );
        return response.data.map(mapEnrollmentSummaryRow);
    });
}

export async function fetchEnrollmentSlotDetails(
    params: EnrollmentSlotParams,
): Promise<PendingEnrollment[]> {
    if (!params.campusId || !params.clubId || params.weekday === undefined) {
        throw new EnrollmentServiceError("缺少校区/社团/星期条件，无法查询报名详情");
    }
    if (params.weekday < 1 || params.weekday > 7) {
        throw new EnrollmentServiceError("星期需在 1-7 之间（1 表示周一）");
    }
    return safeRequest("获取报名名单失败", async () => {
        const search = buildQueryString({
            term_id: params.termId,
            campus_id: params.campusId,
            club_id: params.clubId,
            weekday: String(params.weekday),
        });
        const response = await client.get<EnrollmentListResponse>(
            `/api/enrollments/slots${search}`,
        );
        return response.data.map(mapPendingEnrollment);
    });
}

export async function importEnrollmentExcel(
    options: EnrollmentImportOptions,
): Promise<EnrollmentImportOutcome[]> {
    return safeRequest("导入报名 Excel 失败", async () => {
        const formData = new FormData();
        formData.append("file", options.file);
        if (options.config) {
            formData.append("config", JSON.stringify(options.config));
        }
        const response = await postFormData<EnrollmentImportResponse>(
            "/api/import/enrollments",
            formData,
        );
        return response.outcomes.map(mapEnrollmentImportOutcome);
    });
}

function buildQueryString(values: Record<string, string | undefined>): string {
    const search = new URLSearchParams();
    Object.entries(values).forEach(([key, value]) => {
        if (value && value.length > 0) {
            search.set(key, value);
        }
    });
    const query = search.toString();
    return query ? `?${query}` : "";
}

async function safeRequest<T>(
    context: string,
    executor: () => Promise<T>,
): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof EnrollmentServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new EnrollmentServiceError(`${context}: ${message}`);
    }
}

function mapPendingEnrollment(data: PendingEnrollmentApi): PendingEnrollment {
    return {
        enrollmentId: data.enrollment_id,
        studentId: data.student_id,
        studentName: data.student_name,
        studentCode: toOptional(data.student_code),
        homeroom: data.homeroom,
        campusId: data.campus_id,
        campusName: data.campus_name,
        clubId: data.club_id,
        clubName: data.club_name,
        requestedWeekday: data.requested_weekday,
        status: data.status,
    };
}

function mapEnrollmentSummaryRow(data: EnrollmentSummaryApiRow): EnrollmentSummaryRow {
    return {
        campusId: data.campus_id,
        campusName: data.campus_name,
        clubId: data.club_id,
        clubName: data.club_name,
        requestedWeekday: data.requested_weekday,
        total: Number(data.total),
    };
}

function mapEnrollmentImportOutcome(
    data: EnrollmentImportOutcomeApi,
): EnrollmentImportOutcome {
    return {
        sourceRow: data.source_row,
        draft: data.draft ? mapEnrollmentImportDraft(data.draft) : undefined,
        status: data.status,
        enrollmentId: toOptional(data.enrollment_id),
        message: toOptional(data.message),
    };
}

function mapEnrollmentImportDraft(
    data: EnrollmentImportDraftApi,
): NonNullable<EnrollmentImportOutcome["draft"]> {
    return {
        termId: data.term_id,
        homeroomDisplayName: data.homeroom_display_name,
        studentFullName: data.student_full_name,
        studentCode: toOptional(data.student_code),
        requestedWeekday: data.requested_weekday,
        clubLookupValue: data.club_lookup_value,
        sourceRow: data.source_row,
        rawIdentifier: data.raw_identifier,
    };
}

function toOptional<T>(value: T | null | undefined): T | undefined {
    return value === null || value === undefined ? undefined : value;
}

async function postFormData<T>(path: string, formData: FormData): Promise<T> {
    const response = await fetch(`${API_BASE_URL}${path}`, {
        method: "POST",
        body: formData,
    });
    if (!response.ok) {
        const details = await parseErrorMessage(response);
        throw new Error(`POST ${path} 失败: ${details}`);
    }
    return response.json();
}

async function parseErrorMessage(response: Response): Promise<string> {
    const body = await response.text();
    if (body) {
        try {
            const parsed = JSON.parse(body) as { message?: string };
            if (typeof parsed.message === "string" && parsed.message.trim().length > 0) {
                return parsed.message;
            }
        } catch {
            return body;
        }
        return body;
    }
    return `HTTP ${response.status}`;
}

interface EnrollmentListResponse {
    data: PendingEnrollmentApi[];
}

interface PendingEnrollmentApi {
    enrollment_id: string;
    student_id: string;
    student_name: string;
    student_code: string | null;
    homeroom: string;
    campus_id: string;
    campus_name: string;
    club_id: string;
    club_name: string;
    requested_weekday: number;
    status: PendingEnrollment["status"];
}

interface EnrollmentSummaryResponse {
    data: EnrollmentSummaryApiRow[];
}

interface EnrollmentSummaryApiRow {
    campus_id: string;
    campus_name: string;
    club_id: string;
    club_name: string;
    requested_weekday: number;
    total: number;
}

interface EnrollmentImportResponse {
    outcomes: EnrollmentImportOutcomeApi[];
}

interface EnrollmentImportOutcomeApi {
    source_row: number;
    draft: EnrollmentImportDraftApi | null;
    status: EnrollmentImportOutcome["status"];
    enrollment_id: string | null;
    message: string | null;
}

interface EnrollmentImportDraftApi {
    term_id: string;
    homeroom_display_name: string;
    student_full_name: string;
    student_code: string | null;
    requested_weekday: number;
    club_lookup_value: string;
    source_row: number;
    raw_identifier: string;
}
