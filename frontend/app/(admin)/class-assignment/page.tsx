import { SectionCard } from "@/components/common/SectionCard";
import { ClassAssignmentBoard } from "@/components/class-assignment/ClassAssignmentBoard";
import { fetchEnrollmentSummary } from "@/services/enrollmentService";

export default async function ClassAssignmentPage() {
    const summaryRows = await fetchEnrollmentSummary();

    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">班级分配</h1>
            <SectionCard
                title="社团分班工作台"
                description="复用报名汇总的筛选维度，查看报名名单、维护班级并实时更新学生的分班结果。"
            >
                <ClassAssignmentBoard summaryRows={summaryRows} />
            </SectionCard>
        </div>
    );
}
