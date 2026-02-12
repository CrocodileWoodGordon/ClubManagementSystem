import type {
    AttendanceStatus,
    EnrollmentStatus,
    TuitionWaiverReason,
} from "@/lib/types";

const STATUS_LABELS: Record<EnrollmentStatus, string> = {
    PENDING: "待分班",
    ACTIVE: "已激活",
    DROPPED: "已退课",
    TRANSFERRED_OUT: "已转出",
    TRANSFERRED_IN: "转入待确认",
};

const ATTENDANCE_STATUS_LABELS: Record<AttendanceStatus, string> = {
    PRESENT: "出勤",
    ABSENT: "缺勤",
    EXCUSED: "病假",
    LEAVE: "事假",
};

const WAIVER_REASON_LABELS: Record<TuitionWaiverReason, string> = {
    DROP_WITHIN_GRACE: "三节内退课",
    MANUAL_OVERRIDE: "人工调整",
    TEACHER_BENEFIT: "教师子女优惠",
};

const CURRENCY_FORMATTER = new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
});

export function formatWeekday(dayOfWeek: number) {
    const map = ["一", "二", "三", "四", "五", "六", "日"];
    return `周${map[dayOfWeek - 1] ?? "?"}`;
}

export function formatEnrollmentStatus(status: EnrollmentStatus) {
    return STATUS_LABELS[status] ?? status;
}

export function formatAttendanceStatus(status: AttendanceStatus) {
    return ATTENDANCE_STATUS_LABELS[status] ?? status;
}

export function formatWaiverReason(reason?: TuitionWaiverReason) {
    if (!reason) {
        return "—";
    }
    return WAIVER_REASON_LABELS[reason] ?? reason;
}

export function formatCurrency(value: number) {
    if (!Number.isFinite(value)) {
        return CURRENCY_FORMATTER.format(0);
    }
    return CURRENCY_FORMATTER.format(value);
}
