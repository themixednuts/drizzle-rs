type Options = {
    clientVersion: string;
};
export declare class PrismaClientValidationError extends Error {
    name: string;
    clientVersion: string;
    constructor(message: string, { clientVersion }: Options);
    get [Symbol.toStringTag](): string;
}
export {};
