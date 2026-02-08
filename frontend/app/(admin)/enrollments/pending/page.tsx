import { SectionCard } from "@/components/common/SectionCard";
import { PendingEnrollmentTable } from "@/components/enrollments/PendingEnrollmentTable";
import { fetchPendingEnrollments } from "@/services/enrollmentService";

export default async function PendingEnrollmentPage() {
    const pendingEnrollments = await fetchPendingEnrollments();

    return (
        <div className="space-y-8">
            <SectionCard
                title="待处理名单"
                description="展示仍处于待分班状态的学生，可结合筛选条件进一步分配。"
            >
                <PendingEnrollmentTable rows={pendingEnrollments} />
            </SectionCard>
        </div>
    );
}
