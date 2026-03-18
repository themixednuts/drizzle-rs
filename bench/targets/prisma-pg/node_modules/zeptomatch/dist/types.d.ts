type Node = {
    partial?: boolean;
    regex?: RegExp;
    children: Node[];
};
type Options = {
    partial?: boolean;
};
export type { Node, Options };
