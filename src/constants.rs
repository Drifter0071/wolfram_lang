// Single source of truth for Roblox globals, service lists, and shared data.
// All subsystems (Scope, TypeCK, LuauChecker, Generator, LSP) import from here.

pub const ROBLOX_GLOBALS: &[&str] = &[
    "game",
    "workspace",
    "script",
    "print",
    "warn",
    "error",
    "Players",
    "ReplicatedStorage",
    "ServerScriptService",
    "ServerStorage",
    "StarterPlayer",
    "StarterGui",
    "StarterPack",
    "Lighting",
    "SoundService",
    "RunService",
    "UserInputService",
    "ContextActionService",
    "TweenService",
    "CollectionService",
    "HttpService",
    "TeleportService",
    "MarketplaceService",
    "DataStoreService",
    "MessagingService",
    "PathfindingService",
    "PhysicsService",
    "Teams",
    "Chat",
    "LocalizationService",
    "SocialService",
    "VRService",
    "GroupService",
    "PolicyService",
    "AnalyticsService",
    "AvatarEditorService",
    "BadgeService",
    "MemoryStoreService",
    "TextService",
    "GuiService",
    "HapticService",
    "Enum",
    "Vector3",
    "Vector2",
    "CFrame",
    "UDim2",
    "UDim",
    "Color3",
    "BrickColor",
    "TweenInfo",
    "RaycastParams",
    "Region3",
    "Rect",
    "NumberRange",
    "NumberSequence",
    "ColorSequence",
    "Ray",
    "DateTime",
    "Buffer",
    "Instance",
    "PhysicalProperties",
    "Random",
    "Axes",
    "Faces",
    "math",
    "string",
    "table",
    "os",
    "task",
    "coroutine",
    "debug",
    "utf8",
    "bit32",
    "buffer",
    "typeof",
    "ipairs",
    "pairs",
    "next",
    "rawget",
    "rawset",
    "setmetatable",
    "getmetatable",
    "pcall",
    "xpcall",
    "tostring",
    "tonumber",
    "type",
    "require",
];

pub const SERVER_ONLY_SERVICES: &[&str] = &[
    "ServerScriptService",
    "ServerStorage",
    "DataStoreService",
    "MessagingService",
    "PathfindingService",
];

pub const CLIENT_ONLY_SERVICES: &[&str] = &[
    "UserInputService",
    "GuiService",
    "HapticService",
    "ContextActionService",
    "StarterGui",
];

pub fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                parts.pop();
            }
            _ => parts.push(seg),
        }
    }
    parts.join("/")
}
