export type PrereleaseId = string | number;
export type PreRelease = PrereleaseId[];
export type BuildInfo = string;
export declare class SemanticVersion {
    major: number;
    minor: number;
    patch: number;
    preRelease: PreRelease | null;
    buildInfo: BuildInfo | null;
    constructor(major: number, minor: number, patch: number, preRelease?: PreRelease | null, buildInfo?: BuildInfo | null);
    toString(): string;
    compare(other: SemanticVersion): number;
    clone(): SemanticVersion;
    static parseVersionString(version: string): SemanticVersion;
}
export declare const _MINIMUM_CLI_VERSION: SemanticVersion;
export declare function ensureMinimumVersionOrThrow(versionString?: string): void;
//# sourceMappingURL=version.d.ts.map