export interface Student {
    id: string;
    name: string;
    originalClass: string;
    isTeacherChild: boolean;
}

export interface Term {
    id: string;
    code: string;
    name: string;
    startDate: string;
    endDate: string;
    enrollmentStart: string;
    enrollmentEnd: string;
    isActive: boolean;
}

export interface HomeroomRoster {
    id: string;
    termId: string;
    campusId: string;
    campusName: string;
    academicYear: number;
    displayName: string;
    gradeLabel: string;
    classLabel: string;
    headTeacherName?: string;
    headTeacherPhone?: string;
    notes?: string;
    studentCount: number;
}

export interface RosterStudent {
    id: string;
    homeroomId: string;
    fullName: string;
    studentCode?: string;
    isTeacherChild: boolean;
    primaryGuardianName?: string;
    primaryGuardianPhone?: string;
    status: string;
}

export interface StudentImportSummary {
    jobId: string;
    totalRows: number;
    successRows: number;
    skippedRows: number;
    errors: StudentImportError[];
}

export interface StudentImportError {
    row: number;
    message: string;
}

export interface TeacherChildImportSummary {
    totalRows: number;
    matchedStudents: number;
    updatedStudents: number;
    alreadyMarked: number;
    skippedRows: number;
    duplicateRows: number;
    errors: TeacherChildImportError[];
}

export interface TeacherChildImportError {
    row: number;
    message: string;
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

export interface ClubPlacement {
    campusId: string;
    campusName: string;
    weekday: number;
}

export interface Club {
    id: string;
    code: string;
    name: string;
    description?: string;
    materialFee: number;
    pricePerSession: number;
    graceSessions: number;
    createdAt: string;
    placements?: ClubPlacement[];
}

export interface ClubMember {
    enrollmentId: string;
    studentId: string;
    studentName: string;
    studentCode?: string;
    homeroom: string;
    campusId: string;
    campusName: string;
    termId: string;
    requestedWeekday: number;
    status: EnrollmentStatus;
}

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
    id: string;
    sourceRow: number;
    draft?: EnrollmentImportDraft;
    status: EnrollmentImportStatus;
    enrollmentId?: string;
    message?: string;
}

export type ImportPlaceholderType = "ENROLLMENTS" | "STUDENTS";

export interface ImportPlaceholderConfig {
    importType: ImportPlaceholderType;
    placeholders: string[];
    updatedBy?: string;
    updatedAt: string;
}

export type AttendanceStatus = "PRESENT" | "ABSENT" | "EXCUSED" | "LEAVE";

export interface AttendanceClassOverview {
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
    notes?: string;
}

export interface AttendanceMeeting {
    id: string;
    meetingDate: string;
    sessionNumber: number;
}

export interface AttendanceWorksheet {
    name: string;
    rows: string[][];
    fileName: string;
    fileBase64: string;
    mimeType: string;
}

export interface AttendanceTemplate {
    class: AttendanceClassOverview;
    meetings: AttendanceMeeting[];
    worksheet: AttendanceWorksheet;
}

export interface AttendanceRecord {
    id: string;
    classMeetingId: string;
    meetingDate: string;
    sessionNumber: number;
    enrollmentId: string;
    studentId: string;
    studentName: string;
    studentIdentifier: string;
    status: AttendanceStatus;
    minutesAttended?: number;
    recordedBy?: string;
    recordedAt: string;
}

export interface AttendanceImportSkippedRow {
    sourceRow: number;
    studentIdentifier: string;
    status: AttendanceStatus;
    minutesAttended?: number;
    note?: string;
}

export interface AttendanceImportResult {
    batchId: string;
    inserted: number;
    updated: number;
    skipped: AttendanceImportSkippedRow[];
}

export type TuitionWaiverReason =
    | "DROP_WITHIN_GRACE"
    | "MANUAL_OVERRIDE"
    | "TEACHER_BENEFIT";

export interface FeeBreakdown {
    enrollmentId: string;
    studentId: string;
    classId: string;
    materialFee: number;
    lessonFee: number;
    discountAmount: number;
    attendanceCount: number;
    chargedSessions: number;
    waiveReason?: TuitionWaiverReason;
    remarks?: string;
}


export type StudentBillingItem = FeeBreakdown & {
    clubId: string;
    clubName: string;
    classCode?: string;
};

export interface StudentBillingBundle {
    studentId: string;
    studentName: string;
    studentCode?: string;
    rows: StudentBillingItem[];
}

export interface HomeroomBillingInfo {
    id: string;
    displayName: string;
    campusName: string;
}

export interface HomeroomBillingReport {
    homeroom: HomeroomBillingInfo;
    students: StudentBillingBundle[];
}
