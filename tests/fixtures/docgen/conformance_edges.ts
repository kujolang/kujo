/** Public class docs */
export class TsClient {
    /** Public async call docs */
    public async fetchData(id: string): Promise<string> {
        return id;
    }

    private cacheData(id: string): string {
        return id;
    }
}

/** Exported async helper docs */
export async function tsAsyncHelper(id: string): Promise<string> {
    return id;
}

export function tsPublicWithoutDocs(id: string): string {
    return id;
}
