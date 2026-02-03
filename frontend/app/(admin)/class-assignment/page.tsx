import { BulkAssignmentForm } from "@/components/forms/BulkAssignmentForm";
import { SectionCard } from "@/components/common/SectionCard";
import { StudentTable } from "@/components/tables/StudentTable";
import { DAYS } from "@/constants";

const MOCK_STUDENTS = Array.from({ length: 6 }).map((_, index) => ({
    id: `${index}`,
    name: `学生 ${index + 1}`,
    originalClass: "三年二班",
    status: "待定班",
}));

export default function ClassAssignmentPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">手动分班</h1>
            <SectionCard
                title="筛选条件"
                description="按社团、星期过滤待分班学生，后续会调用后端 API"
            >
                <div className="flex flex-wrap gap-3 text-sm text-slate-600">
                    {DAYS.map((day) => (
                        <span key={day.value} className="rounded-full border border-slate-200 px-3 py-1">
                            {day.label}
                        </span>
                    ))}
                </div>
            </SectionCard>
            <BulkAssignmentForm onSubmit={async () => {}} />
            <SectionCard title="待定班学生">
                <StudentTable students={MOCK_STUDENTS} />
            </SectionCard>
        </div>
    );
}
