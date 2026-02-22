import { ApiClient } from "@/lib/api/client";
import type {
    AttendanceImportResult,
    AttendanceRecord,
    AttendanceTemplate,
} from "@/lib/types";

const client = new ApiClient();
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "/api";

export class AttendanceServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "AttendanceServiceError";
    }
}

export interface AttendanceHistoryParams {
    classId: string;
    classMeetingId?: string;
}

export interface AttendanceImportPayload {
    classMeetingId: string;
    file: File;
    recordedBy?: string;
    ignoredIdentifiers?: string[];
}

export interface AttendanceTemplateOptions {
    startWeek?: number;
    endWeek?: number;
}

export async function fetchAttendanceTemplate(
    classId: string,
    options: AttendanceTemplateOptions = {},
): Promise<AttendanceTemplate> {
    if (!classId) {
        throw new AttendanceServiceError("请先选择班级");
    }
    return safeRequest("获取考勤模板失败", async () => {
        const query = buildQueryString({
            start_week: options.startWeek ? String(options.startWeek) : undefined,
            end_week: options.endWeek ? String(options.endWeek) : undefined,
        });
        const response = await client.get<AttendanceTemplateApiResponse>(
            `/api/attendance/template/${classId}${query}`,
        );
        return mapTemplate(response);
    });
}

export async function fetchAttendanceHistory(
    params: AttendanceHistoryParams,
): Promise<AttendanceRecord[]> {
    if (!params.classId) {
        throw new AttendanceServiceError("请先选择班级再查看考勤记录");
    }
    return safeRequest("加载考勤记录失败", async () => {
        const query = buildQueryString({
            class_id: params.classId,
            class_meeting_id: params.classMeetingId,
        });
        const response = await client.get<AttendanceHistoryResponse>(`/api/attendance${query}`);
        return response.data.map(mapRecord);
    });
}

export async function importAttendanceRecords(
    payload: AttendanceImportPayload,
): Promise<AttendanceImportResult> {
    if (!payload.file) {
        throw new AttendanceServiceError("请先选择 Excel 文件");
    }
    return safeRequest("导入考勤 Excel 失败", async () => {
        const formData = new FormData();
        formData.append("file", payload.file);
        formData.append("class_meeting_id", payload.classMeetingId);
        if (payload.recordedBy) {
            formData.append("recorded_by", payload.recordedBy);
        }
        if (payload.ignoredIdentifiers && payload.ignoredIdentifiers.length > 0) {
            const filtered = payload.ignoredIdentifiers
                .map((value) => value.trim())
                .filter((value) => value.length > 0);
            if (filtered.length > 0) {
                formData.append("ignored_identifiers", JSON.stringify(filtered));
            }
        }

        const response = await fetch(`${API_BASE_URL}/api/attendance/import`, {
            method: "POST",
            body: formData,
        });
        if (!response.ok) {
            const message = await parseError(response);
            throw new AttendanceServiceError(message);
        }
        const data = (await response.json()) as AttendanceImportApiResponse;
        return mapImportResult(data);
    });
}

function mapTemplate(payload: AttendanceTemplateApiResponse): AttendanceTemplate {
    return {
        class: {
            id: payload.class.id,
            termId: payload.class.term_id,
            campusId: payload.class.campus_id,
            clubId: payload.class.club_id,
            classCode: payload.class.class_code,
            weekday: payload.class.weekday,
            startTime: payload.class.start_time,
            endTime: payload.class.end_time,
            location: toOptional(payload.class.location),
            capacity: payload.class.capacity ?? undefined,
            notes: toOptional(payload.class.notes),
        },
        meetings: payload.meetings.map((meeting) => ({
            id: meeting.id,
            meetingDate: meeting.meeting_date,
            sessionNumber: meeting.session_number,
        })),
        worksheet: {
            name: payload.worksheet.name,
            rows: payload.worksheet.rows,
            fileName: payload.worksheet.file_name,
            fileBase64: payload.worksheet.file_base64,
            mimeType: payload.worksheet.mime_type,
        },
    };
}

function mapRecord(payload: AttendanceRecordApi): AttendanceRecord {
    return {
        id: payload.id,
        classMeetingId: payload.class_meeting_id,
        meetingDate: payload.meeting_date,
        sessionNumber: payload.session_number,
        enrollmentId: payload.enrollment_id,
        studentId: payload.student_id,
        studentName: payload.student_name,
        studentIdentifier: payload.student_identifier,
        status: payload.status,
        minutesAttended: payload.minutes_attended ?? undefined,
        recordedBy: toOptional(payload.recorded_by),
        recordedAt: payload.recorded_at,
    };
}

function mapImportResult(payload: AttendanceImportApiResponse): AttendanceImportResult {
    return {
        batchId: payload.batch_id,
        inserted: payload.inserted,
        updated: payload.updated,
        skipped: payload.skipped.map((row) => ({
            sourceRow: row.source_row,
            studentIdentifier: row.student_identifier,
            status: row.status,
            minutesAttended: row.minutes_attended ?? undefined,
            note: toOptional(row.note),
        })),
    };
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

async function safeRequest<T>(context: string, executor: () => Promise<T>): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof AttendanceServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new AttendanceServiceError(`${context}: ${message}`);
    }
}

async function parseError(response: Response): Promise<string> {
    const body = await response.text();
    if (body.length > 0) {
        try {
            const parsed = JSON.parse(body) as { message?: string };
            if (parsed.message && parsed.message.trim().length > 0) {
                return parsed.message;
            }
        } catch {
            return body;
        }
        return body;
    }
    return `HTTP ${response.status}`;
}

function toOptional<T>(value: T | null | undefined): T | undefined {
    return value === null || value === undefined ? undefined : value;
}

interface AttendanceTemplateApiResponse {
    class: AttendanceClassApi;
    meetings: AttendanceMeetingApi[];
    worksheet: AttendanceWorksheetApi;
}

interface AttendanceClassApi {
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
    notes: string | null;
}

interface AttendanceMeetingApi {
    id: string;
    meeting_date: string;
    session_number: number;
}

interface AttendanceWorksheetApi {
    name: string;
    rows: string[][];
    file_name: string;
    file_base64: string;
    mime_type: string;
}

interface AttendanceHistoryResponse {
    data: AttendanceRecordApi[];
}

interface AttendanceRecordApi {
    id: string;
    class_meeting_id: string;
    meeting_date: string;
    session_number: number;
    enrollment_id: string;
    student_id: string;
    student_name: string;
    student_identifier: string;
    status: AttendanceRecord["status"];
    minutes_attended: number | null;
    recorded_by: string | null;
    recorded_at: string;
}

interface AttendanceImportApiResponse {
    batch_id: string;
    inserted: number;
    updated: number;
    skipped: AttendanceImportSkippedRowApi[];
}

interface AttendanceImportSkippedRowApi {
    source_row: number;
    student_identifier: string;
    status: AttendanceRecord["status"];
    minutes_attended: number | null;
    note: string | null;
}
