import { SectionCard } from "@/components/common/SectionCard";
import { TermSettingsPanel } from "@/components/settings/TermSettingsPanel";
import { fetchTerms } from "@/services/termService";

export default async function TermSettingsPage() {
    const terms = await fetchTerms();

    return (
        <div className="space-y-8">
            <SectionCard
                title="学期管理"
                description="维护学期基础信息，支持创建、编辑、删除以及切换当前学期。"
            >
                <TermSettingsPanel initialTerms={terms} />
            </SectionCard>
        </div>
    );
}
