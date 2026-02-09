import { ClubManagementWorkspace } from "@/components/clubs/ClubManagementWorkspace";
import type { Club } from "@/lib/types";
import { ApiClient } from "@/lib/api/client";

interface ClubListApiResponse {
    data: ClubApi[];
}

interface ClubApi {
    id: string;
    code: string;
    name: string;
    description: string | null;
    material_fee: number;
    price_per_session: number;
    grace_sessions: number;
    created_at: string;
}

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

export default async function EnrollmentClubsPage() {
    const client = new ApiClient();
    const errors: string[] = [];

    let clubs: ClubApi[] = [];
    let terms: TermApi[] = [];
    let campuses: CampusApi[] = [];

    try {
        const response = await client.get<ClubListApiResponse>("/api/clubs");
        clubs = response.data;
    } catch (error) {
        errors.push(parseError("加载社团数据失败", error));
    }

    try {
        terms = await client.get<TermApi[]>("/api/admin/terms");
    } catch (error) {
        errors.push(parseError("加载学期数据失败", error));
    }

    try {
        campuses = await client.get<CampusApi[]>("/api/admin/campuses");
    } catch (error) {
        errors.push(parseError("加载校区数据失败", error));
    }

    const mappedClubs: Club[] = clubs.map((club) => ({
        id: club.id,
        code: club.code,
        name: club.name,
        description: club.description ?? undefined,
        materialFee: Number(club.material_fee),
        pricePerSession: Number(club.price_per_session),
        graceSessions: club.grace_sessions,
        createdAt: club.created_at,
    }));
    const termOptions = terms.map((term) => ({
        id: term.id,
        code: term.code,
        name: term.name,
        isActive: term.is_active,
    }));
    const campusOptions = campuses.map((campus) => ({
        id: campus.id,
        name: campus.name,
        shortName: campus.short_name ?? undefined,
    }));
    const activeTerm = termOptions.find((term) => term.isActive);

    return (
        <div className="space-y-8">
            <ClubManagementWorkspace
                initialClubs={mappedClubs}
                terms={termOptions}
                campuses={campusOptions}
                defaultTermId={activeTerm?.id}
                initialError={errors[0]}
            />
        </div>
    );
}

function parseError(prefix: string, error: unknown): string {
    if (error instanceof Error) {
        return `${prefix}: ${error.message}`;
    }
    return `${prefix}: ${String(error)}`;
}
