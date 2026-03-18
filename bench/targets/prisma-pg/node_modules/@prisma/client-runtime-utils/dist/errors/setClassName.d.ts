/**
 * Used for preserving class names for minified class instances
 * Useful for error objects and other classes where public name
 * actually matters
 *
 * @param classObject
 * @param name
 */
export declare function setClassName(classObject: Function, name: string): void;
