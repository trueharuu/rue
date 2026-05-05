import type { PermissionLevel } from "./level";

export function missing_permissions(
    required: PermissionLevel,
    actual: PermissionLevel,
): string {
    return `this command requests permission level '${required}' but you are '${actual}'`;
}
