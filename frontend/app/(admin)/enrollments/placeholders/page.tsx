import { SectionCard } from "@/components/common/SectionCard";
import { PlaceholderSettingsPanel } from "@/components/enrollments/PlaceholderSettingsPanel";
import { fetchImportPlaceholders } from "@/services/importPlaceholderService";

export default async function EnrollmentPlaceholderSettingsPage() {
    const configs = await fetchImportPlaceholders();

    return (
        <div className="space-y-8">
            <SectionCard
                title="占位文本设置"
                description="统一维护 Excel 中代表“跳过/空”报名的字符串，调整后立即生效。"
            >
                <PlaceholderSettingsPanel initialConfigs={configs} />
            </SectionCard>
        </div>
    );
}
