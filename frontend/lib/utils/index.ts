export function formatWeekday(dayOfWeek: number) {
    const map = ["一", "二", "三", "四", "五", "六", "日"];
    return `周${map[dayOfWeek - 1] ?? "?"}`;
}
