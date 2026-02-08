import { SectionCard } from "@/components/common/SectionCard";
import { EnrollmentSlotExplorer } from "@/components/enrollments/EnrollmentSlotExplorer";
import { fetchEnrollmentSummary } from "@/services/enrollmentService";

export default async function EnrollmentSlotPage() {
    const summaryRows = await fetchEnrollmentSummary();

    return (
        <div className="space-y-8">
            <SectionCard
                title="筛选报名名单"
                description="基于校区 / 社团 / 星期快速筛选报名学生，点击查询后再加载完整名单。"
            >
                <EnrollmentSlotExplorer summaryRows={summaryRows} />
            </SectionCard>
        </div>
    );
}
