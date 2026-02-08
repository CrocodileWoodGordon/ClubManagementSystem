import { ApiClient } from "@/lib/api/client";
import type { ImportPlaceholderConfig, ImportPlaceholderType } from "@/lib/types";

const client = new ApiClient();

export class ImportPlaceholderServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "ImportPlaceholderServiceError";
    }
}

interface PlaceholderConfigApi {
    import_type: ImportPlaceholderType;
    placeholders: string[];
    updated_by?: string;
    updated_at: string;
}

interface PlaceholderListResponse {
    data: PlaceholderConfigApi[];
}

interface PlaceholderSingleResponse {
    data: PlaceholderConfigApi;
}

function mapPlaceholderConfig(data: PlaceholderConfigApi): ImportPlaceholderConfig {
    return {
        importType: data.import_type,
        placeholders: data.placeholders,
        updatedBy: data.updated_by,
        updatedAt: data.updated_at,
    };
}

export async function fetchImportPlaceholders(): Promise<ImportPlaceholderConfig[]> {
    try {
        const response = await client.get<PlaceholderListResponse>("/api/import/placeholders");
        return response.data.map(mapPlaceholderConfig);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new ImportPlaceholderServiceError(`获取占位文本失败: ${message}`);
    }
}

export async function updateImportPlaceholders(
    importType: ImportPlaceholderType,
    placeholders: string[],
    updatedBy = "admin-ui",
): Promise<ImportPlaceholderConfig> {
    try {
        const response = await client.put<PlaceholderSingleResponse>(
            `/api/import/placeholders/${importType}`,
            {
                placeholders,
                updated_by: updatedBy,
            },
        );
        return mapPlaceholderConfig(response.data);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new ImportPlaceholderServiceError(`更新占位文本失败: ${message}`);
    }
}
