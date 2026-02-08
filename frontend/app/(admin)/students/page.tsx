import { StudentRosterDashboard } from "@/components/students/StudentRosterDashboard";
import type {
    CampusOption,
    TermOption,
} from "@/components/students/StudentRosterWorkspace";
import { ApiClient } from "@/lib/api/client";

interface TermApi {
    id: string;
    code: string;
    name: string;
    is_active: boolean;
}

interface CampusApi {
    id: string;
    name: string;
    short_name: string | null;
}

export default async function StudentsPage() {
    const client = new ApiClient();
    const [terms, campuses] = await Promise.all([
        client.get<TermApi[]>("/api/admin/terms"),
        client.get<CampusApi[]>("/api/admin/campuses"),
    ]);
    const mappedTerms: TermOption[] = terms.map((term) => ({
        id: term.id,
        code: term.code,
        name: term.name,
        isActive: term.is_active,
    }));
    const mappedCampuses: CampusOption[] = campuses.map((campus) => ({
        id: campus.id,
        name: campus.name,
        shortName: campus.short_name ?? undefined,
    }));
    const activeTerm = mappedTerms.find((term) => term.isActive);

    return (
        <StudentRosterDashboard
            terms={mappedTerms}
            campuses={mappedCampuses}
            defaultTermId={activeTerm?.id}
        />
    );
}
