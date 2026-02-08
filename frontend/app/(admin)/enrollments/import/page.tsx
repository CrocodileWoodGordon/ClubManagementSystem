import { SectionCard } from "@/components/common/SectionCard";
import { EnrollmentImportPanel } from "@/components/upload/EnrollmentImportPanel";

export default function EnrollmentImportPage() {
    return (
        <div className="space-y-8">
            <SectionCard
                title="导入问卷星报名"
                description="上传 Excel 后会即时解析并返回逐行处理结果"
            >
                <EnrollmentImportPanel />
            </SectionCard>
        </div>
    );
}
