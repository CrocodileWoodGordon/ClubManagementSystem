import { ApiClient } from "@/lib/api/client";
import type {
    ColumnReference,
    HomeroomRoster,
    RosterStudent,
    StudentImportSummary,
    TeacherChildImportSummary,
} from "@/lib/types";

const client = new ApiClient();
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export class StudentRosterServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "StudentRosterServiceError";
    }
}

export interface HomeroomQueryParams {
    termId?: string;
    campusId?: string;
    search?: string;
}

export interface StudentQueryParams {
    termId?: string;
}

export interface UpdateHomeroomInput {
    termId?: string;
    displayName?: string;
    gradeLabel?: string;
    classLabel?: string;
    headTeacherName?: string;
    headTeacherPhone?: string;
    notes?: string;
}

export interface CreateRosterStudentInput {
    termId?: string;
    fullName: string;
    studentCode?: string;
    isTeacherChild?: boolean;
    primaryGuardianName?: string;
    primaryGuardianPhone?: string;
}

export interface UpdateRosterStudentInput {
    termId?: string;
    homeroomId?: string;
    fullName?: string;
    studentCode?: string;
    isTeacherChild?: boolean;
    primaryGuardianName?: string;
    primaryGuardianPhone?: string;
    status?: string;
}

export interface CloneRosterInput {
    sourceTermId: string;
    targetTermId: string;
    campusId?: string;
}

export interface CloneRosterSummary {
    copiedHomerooms: number;
    copiedStudents: number;
}

export type TeacherChildImportMode = "SPLIT" | "COMBINED";

export interface TeacherChildImportConfig {
    mode?: TeacherChildImportMode;
    classColumn?: ColumnReference;
    studentColumn?: ColumnReference;
    combinedColumn?: ColumnReference;
}

export interface TeacherChildImportRequest {
    termId: string;
    campusId: string;
    file: File;
    config?: TeacherChildImportConfig;
}

export async function fetchHomerooms(
    params?: HomeroomQueryParams,
): Promise<HomeroomRoster[]> {
    return safeRequest("获取班级名册失败", async () => {
        const query = buildQueryString({
            term_id: params?.termId,
            campus_id: params?.campusId,
            search: params?.search,
        });
        const response = await client.get<HomeroomListApiResponse>(
            `/api/students/homerooms${query}`,
        );
        return response.data.map(mapHomeroom);
    });
}

export async function fetchHomeroomDetail(
    homeroomId: string,
    params?: StudentQueryParams,
): Promise<HomeroomRoster> {
    return safeRequest("读取班级详情失败", async () => {
        const query = buildQueryString({ term_id: params?.termId });
        const response = await client.get<HomeroomDetailApiResponse>(
            `/api/students/homerooms/${homeroomId}${query}`,
        );
        return mapHomeroom(response.data);
    });
}

export async function updateHomeroom(
    homeroomId: string,
    payload: UpdateHomeroomInput,
): Promise<HomeroomRoster> {
    return safeRequest("更新班级信息失败", async () => {
        const query = buildQueryString({ term_id: payload.termId });
        const response = await client.put<HomeroomDetailApiResponse>(
            `/api/students/homerooms/${homeroomId}${query}`,
            {
                display_name: sanitize(payload.displayName),
                grade_label: sanitize(payload.gradeLabel),
                class_label: sanitize(payload.classLabel),
                head_teacher_name: sanitize(payload.headTeacherName),
                head_teacher_phone: sanitize(payload.headTeacherPhone),
                notes: sanitize(payload.notes),
            },
        );
        return mapHomeroom(response.data);
    });
}

export async function fetchHomeroomStudents(
    homeroomId: string,
    params?: StudentQueryParams,
): Promise<RosterStudent[]> {
    return safeRequest("获取学生列表失败", async () => {
        const query = buildQueryString({ term_id: params?.termId });
        const response = await client.get<StudentListApiResponse>(
            `/api/students/homerooms/${homeroomId}/students${query}`,
        );
        return response.data.map(mapRosterStudent);
    });
}

export async function createStudent(
    homeroomId: string,
    payload: CreateRosterStudentInput,
): Promise<RosterStudent> {
    return safeRequest("新增学生失败", async () => {
        const query = buildQueryString({ term_id: payload.termId });
        const response = await client.post<StudentDetailApiResponse>(
            `/api/students/homerooms/${homeroomId}/students${query}`,
            {
                full_name: payload.fullName,
                student_code: sanitize(payload.studentCode),
                is_teacher_child: Boolean(payload.isTeacherChild),
                primary_guardian_name: sanitize(payload.primaryGuardianName),
                primary_guardian_phone: sanitize(payload.primaryGuardianPhone),
            },
        );
        return mapRosterStudent(response.data);
    });
}

export async function updateStudent(
    studentId: string,
    payload: UpdateRosterStudentInput,
): Promise<RosterStudent> {
    return safeRequest("更新学生资料失败", async () => {
        const query = buildQueryString({ term_id: payload.termId });
        const response = await client.put<StudentDetailApiResponse>(
            `/api/students/${studentId}${query}`,
            {
                homeroom_id: payload.homeroomId,
                full_name: sanitize(payload.fullName),
                student_code: sanitize(payload.studentCode),
                is_teacher_child: payload.isTeacherChild,
                primary_guardian_name: sanitize(payload.primaryGuardianName),
                primary_guardian_phone: sanitize(payload.primaryGuardianPhone),
                status: sanitize(payload.status),
            },
        );
        return mapRosterStudent(response.data);
    });
}

export async function deleteStudent(
    studentId: string,
    params?: StudentQueryParams,
): Promise<void> {
    return safeRequest("删除学生失败", async () => {
        const query = buildQueryString({ term_id: params?.termId });
        await client.delete(`/api/students/${studentId}${query}`);
    });
}

export async function cloneRoster(
    payload: CloneRosterInput,
): Promise<CloneRosterSummary> {
    return safeRequest("复用学生名册失败", async () => {
        const response = await client.post<CloneRosterApiResponse>(
            "/api/students/homerooms/clone",
            {
                source_term_id: payload.sourceTermId,
                target_term_id: payload.targetTermId,
                campus_id: payload.campusId,
            },
        );
        return {
            copiedHomerooms: response.data.copied_homerooms,
            copiedStudents: response.data.copied_students,
        };
    });
}

export async function importStudentExcel(file: File): Promise<StudentImportSummary> {
    return safeRequest("导入学生名单失败", async () => {
        const formData = new FormData();
        formData.append("file", file);
        const response = await uploadFormData<StudentImportResponse>(
            "/api/import/students",
            formData,
        );
        return mapStudentImportSummary(response.summary);
    });
}

export async function importTeacherChildrenExcel(
    payload: TeacherChildImportRequest,
): Promise<TeacherChildImportSummary> {
    return safeRequest("导入教师子女名单失败", async () => {
        if (!payload.termId || !payload.campusId) {
            throw new StudentRosterServiceError("请先选择学期与校区");
        }
        const formData = new FormData();
        formData.append("file", payload.file);
        if (payload.config) {
            formData.append("config", JSON.stringify(payload.config));
        }
        const query = buildQueryString({
            term_id: payload.termId,
            campus_id: payload.campusId,
        });
        const response = await uploadFormData<TeacherChildImportApiResponse>(
            `/api/students/teacher-children/import${query}`,
            formData,
        );
        return mapTeacherChildImportSummary(response.data);
    });
}

async function safeRequest<T>(
    context: string,
    executor: () => Promise<T>,
): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof StudentRosterServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new StudentRosterServiceError(`${context}: ${message}`);
    }
}

function buildQueryString(values: Record<string, string | undefined | null>): string {
    const params = new URLSearchParams();
    Object.entries(values).forEach(([key, value]) => {
        if (value !== undefined && value !== null && value.length > 0) {
            params.set(key, value);
        }
    });
    const query = params.toString();
    return query ? `?${query}` : "";
}

function sanitize(value?: string): string | undefined {
    if (typeof value !== "string") {
        return undefined;
    }
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
}

async function uploadFormData<T>(path: string, formData: FormData): Promise<T> {
    const response = await fetch(`${API_BASE_URL}${path}`, {
        method: "POST",
        body: formData,
    });
    if (!response.ok) {
        const details = await parseErrorMessage(response);
        throw new StudentRosterServiceError(`POST ${path} 失败: ${details}`);
    }
    return response.json();
}

async function parseErrorMessage(response: Response): Promise<string> {
    const body = await response.text();
    if (!body) {
        return `HTTP ${response.status}`;
    }
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

function mapHomeroom(payload: HomeroomApi): HomeroomRoster {
    return {
        id: payload.id,
        termId: payload.term_id,
        campusId: payload.campus_id,
        campusName: payload.campus_name,
        academicYear: payload.academic_year,
        displayName: payload.display_name,
        gradeLabel: payload.grade_label,
        classLabel: payload.class_label,
        headTeacherName: payload.head_teacher_name ?? undefined,
        headTeacherPhone: payload.head_teacher_phone ?? undefined,
        notes: payload.notes ?? undefined,
        studentCount: payload.student_count,
    };
}

function mapRosterStudent(payload: StudentRecordApi): RosterStudent {
    return {
        id: payload.id,
        homeroomId: payload.homeroom_id,
        fullName: payload.full_name,
        studentCode: payload.student_code ?? undefined,
        isTeacherChild: payload.is_teacher_child,
        primaryGuardianName: payload.primary_guardian_name ?? undefined,
        primaryGuardianPhone: payload.primary_guardian_phone ?? undefined,
        status: payload.status,
    };
}

function mapStudentImportSummary(payload: StudentImportSummaryApi): StudentImportSummary {
    return {
        jobId: payload.job_id,
        totalRows: payload.total_rows,
        successRows: payload.success_rows,
        skippedRows: payload.skipped_rows,
        errors: payload.errors.map((item) => ({
            row: item.row,
            message: item.message,
        })),
    };
}

function mapTeacherChildImportSummary(
    payload: TeacherChildImportSummaryApi,
): TeacherChildImportSummary {
    return {
        totalRows: payload.total_rows,
        matchedStudents: payload.matched_students,
        updatedStudents: payload.updated_students,
        alreadyMarked: payload.already_marked,
        skippedRows: payload.skipped_rows,
        duplicateRows: payload.duplicate_rows,
        errors: payload.errors.map((item) => ({
            row: item.row,
            message: item.message,
        })),
    };
}

interface HomeroomListApiResponse {
    data: HomeroomApi[];
}

interface HomeroomDetailApiResponse {
    data: HomeroomApi;
}

interface StudentListApiResponse {
    data: StudentRecordApi[];
}

interface StudentDetailApiResponse {
    data: StudentRecordApi;
}

interface CloneRosterApiResponse {
    data: {
        copied_homerooms: number;
        copied_students: number;
    };
}

interface HomeroomApi {
    id: string;
    term_id: string;
    campus_id: string;
    campus_name: string;
    academic_year: number;
    display_name: string;
    grade_label: string;
    class_label: string;
    head_teacher_name: string | null;
    head_teacher_phone: string | null;
    notes: string | null;
    student_count: number;
}

interface StudentRecordApi {
    id: string;
    homeroom_id: string;
    full_name: string;
    student_code: string | null;
    is_teacher_child: boolean;
    primary_guardian_name: string | null;
    primary_guardian_phone: string | null;
    status: string;
}

interface StudentImportResponse {
    summary: StudentImportSummaryApi;
}

interface StudentImportSummaryApi {
    job_id: string;
    total_rows: number;
    success_rows: number;
    skipped_rows: number;
    errors: Array<{
        row: number;
        message: string;
    }>;
}

interface TeacherChildImportApiResponse {
    data: TeacherChildImportSummaryApi;
}

interface TeacherChildImportSummaryApi {
    total_rows: number;
    matched_students: number;
    updated_students: number;
    already_marked: number;
    skipped_rows: number;
    duplicate_rows: number;
    errors: Array<{
        row: number;
        message: string;
    }>;
}
