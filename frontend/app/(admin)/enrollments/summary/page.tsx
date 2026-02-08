import { SectionCard } from "@/components/common/SectionCard";
import { EnrollmentSummaryTable } from "@/components/enrollments/EnrollmentSummaryTable";
import { fetchEnrollmentSummary } from "@/services/enrollmentService";

export default async function EnrollmentSummaryPage() {
    const summaryRows = await fetchEnrollmentSummary();

    return (
        <div className="space-y-8">
            <SectionCard
                title="报名汇总"
                description="按校区 / 社团 / 星期聚合的报名概览，可作为分班与排课的前置依据。"
            >
                <EnrollmentSummaryTable rows={summaryRows} />
            </SectionCard>
        </div>
    );
}
