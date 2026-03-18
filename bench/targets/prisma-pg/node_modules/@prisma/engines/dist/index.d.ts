import type { BinaryPaths } from '@prisma/fetch-engine';
import type { DownloadOptions } from '@prisma/fetch-engine';
import { enginesVersion } from '@prisma/engines-version';

export { enginesVersion }

export declare function ensureNeededBinariesExist({ download }: EnsureSomeBinariesExistInput): Promise<void>;

declare type EnsureSomeBinariesExistInput = {
    download: (options: DownloadOptions) => Promise<BinaryPaths>;
};

export declare function getEnginesPath(): string;

export { }
