export interface Student {
    id: string;
    name: string;
    originalClass: string;
    isTeacherChild: boolean;
}

export type ClassStatus = "PLANNED" | "ACTIVE" | "ARCHIVED";

export interface ClassInstance {
    id: string;
    termId: string;
    campusId: string;
    clubId: string;
    classCode: string;
    weekday: number;
    startTime: string;
    endTime: string;
    location?: string;
    capacity?: number;
    status: ClassStatus;
    notes?: string;
    assignedCount: number;
}

export type EnrollmentStatus = "PENDING" | "ACTIVE" | "DROPPED" | "TRANSFERRED_OUT" | "TRANSFERRED_IN";

export interface PendingEnrollment {
    enrollmentId: string;
    studentId: string;
    studentName: string;
    studentCode?: string;
    homeroom: string;
    campusId: string;
    campusName: string;
    clubId: string;
    clubName: string;
    requestedWeekday: number;
    status: EnrollmentStatus;
    classId?: string;
    classCode?: string;
}

export interface EnrollmentSummaryRow {
    campusId: string;
    campusName: string;
    clubId: string;
    clubName: string;
    requestedWeekday: number;
    total: number;
}

export type ColumnReference = string | number;

export interface EnrollmentImportConfig {
    studentColumn?: ColumnReference;
    weekdayColumns?: Record<number, ColumnReference>;
}

export type EnrollmentImportStatus = "PENDING" | "CREATED" | "SKIPPED" | "FAILED";

export interface EnrollmentImportDraft {
    termId: string;
    homeroomDisplayName: string;
    studentFullName: string;
    studentCode?: string;
    requestedWeekday: number;
    clubLookupValue: string;
    sourceRow: number;
    rawIdentifier: string;
}

export interface EnrollmentImportOutcome {
    sourceRow: number;
    draft?: EnrollmentImportDraft;
    status: EnrollmentImportStatus;
    enrollmentId?: string;
    message?: string;
}
