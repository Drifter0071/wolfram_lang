/**
 * luau_datatypes.ts — Roblox Luau built-in datatype definitions
 *
 * These types are globals in the Roblox Luau VM but are NOT Instance types.
 * They do not appear in the roblox.wold API dump, so we define them here
 * and inject them into bindings after loading the .wold file.
 */

export interface LuauDatatype {
	name: string;
	description: string;
	extends: string | null;
	tags: string[];
	properties: LuauProperty[];
	methods: LuauMethod[];
	staticMethods: LuauMethod[];
}

export interface LuauProperty {
	name: string;
	type: string;
	rw: boolean;
	description: string;
}

export interface LuauMethod {
	name: string;
	params: LuauParam[];
	returns: string;
	description: string;
}

interface LuauParam {
	name: string;
	type: string;
}

const p = (name: string, type: string): LuauParam => ({ name, type });
const prop = (name: string, type: string, rw: boolean, description: string): LuauProperty =>
	({ name, type, rw, description });
const method = (name: string, params: LuauParam[], returns: string, description: string): LuauMethod =>
	({ name, params, returns, description });
const statMethod = method; // alias for clarity

// ── Luau built-in datatypes ────────────────────────────────────────────────

export const LUAU_DATATYPES: LuauDatatype[] = [

	// ── Vector3 ────────────────────────────────────────────────────────────
	{
		name: "Vector3",
		description: "A 3D vector with X, Y, Z components. Immutable value type.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("X", "number", false, "The X component"),
			prop("Y", "number", false, "The Y component"),
			prop("Z", "number", false, "The Z component"),
			prop("Magnitude", "number", false, "The length of this vector"),
			prop("Unit", "Vector3", false, "A normalized copy of this vector"),
			prop("Zero", "Vector3", false, "The zero vector (0, 0, 0)"),
			prop("one", "Vector3", false, "The vector (1, 1, 1)"),
			prop("xAxis", "Vector3", false, "The unit vector (1, 0, 0)"),
			prop("yAxis", "Vector3", false, "The unit vector (0, 1, 0)"),
			prop("zAxis", "Vector3", false, "The unit vector (0, 0, 1)"),
		],
		methods: [
			method("Abs", [], "Vector3", "Returns a vector with the absolute value of each component"),
			method("Ceil", [], "Vector3", "Returns a vector with each component rounded up"),
			method("Cross", [p("other", "Vector3")], "Vector3", "Returns the cross product"),
			method("Dot", [p("other", "Vector3")], "number", "Returns the dot product"),
			method("Floor", [], "Vector3", "Returns a vector with each component rounded down"),
			method("Lerp", [p("goal", "Vector3"), p("alpha", "number")], "Vector3", "Linearly interpolates between two vectors"),
			method("Max", [p("other", "Vector3")], "Vector3", "Returns component-wise maximum"),
			method("Min", [p("other", "Vector3")], "Vector3", "Returns component-wise minimum"),
			method("Sign", [], "Vector3", "Returns component-wise sign (-1, 0, or 1)"),
			method("FuzzyEq", [p("other", "Vector3"), p("epsilon", "number")], "boolean", "Checks approximate equality within epsilon"),
		],
		staticMethods: [
			statMethod("new", [p("x", "number"), p("y", "number"), p("z", "number")], "Vector3", "Creates a new Vector3"),
			statMethod("FromAxis", [p("axis", "Enum.Axis")], "Vector3", "Creates a unit vector from an axis enum"),
			statMethod("FromNormalId", [p("normalId", "Enum.NormalId")], "Vector3", "Creates a unit vector from a normal ID"),
			statMethod("FromEulerAnglesXYZ", [p("x", "number"), p("y", "number"), p("z", "number")], "Vector3", "Converts Euler angles to a direction vector"),
			statMethod("FromEulerAnglesYXZ", [p("y", "number"), p("x", "number"), p("z", "number")], "Vector3", "Converts Euler angles (Y-X-Z order) to a direction vector"),
		],
	},

	// ── Vector2 ────────────────────────────────────────────────────────────
	{
		name: "Vector2",
		description: "A 2D vector with X and Y components. Immutable value type.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("X", "number", false, "The X component"),
			prop("Y", "number", false, "The Y component"),
			prop("Magnitude", "number", false, "The length of this vector"),
			prop("Unit", "Vector2", false, "A normalized copy of this vector"),
			prop("zero", "Vector2", false, "The zero vector (0, 0)"),
			prop("one", "Vector2", false, "The vector (1, 1)"),
		],
		methods: [
			method("Abs", [], "Vector2", "Returns a vector with absolute values"),
			method("Ceil", [], "Vector2", "Returns a vector with each component rounded up"),
			method("Cross", [p("other", "Vector2")], "number", "Returns the 2D cross product (scalar)"),
			method("Dot", [p("other", "Vector2")], "number", "Returns the dot product"),
			method("Floor", [], "Vector2", "Returns a vector with each component rounded down"),
			method("Lerp", [p("goal", "Vector2"), p("alpha", "number")], "Vector2", "Linearly interpolates between two vectors"),
			method("Max", [p("other", "Vector2")], "Vector2", "Returns component-wise maximum"),
			method("Min", [p("other", "Vector2")], "Vector2", "Returns component-wise minimum"),
			method("Sign", [], "Vector2", "Returns component-wise sign (-1, 0, or 1)"),
			method("FuzzyEq", [p("other", "Vector2"), p("epsilon", "number")], "boolean", "Checks approximate equality"),
		],
		staticMethods: [
			statMethod("new", [p("x", "number"), p("y", "number")], "Vector2", "Creates a new Vector2"),
		],
	},

	// ── CFrame ─────────────────────────────────────────────────────────────
	{
		name: "CFrame",
		description: "A coordinate frame: a 3D position and rotation matrix. Immutable value type.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("X", "number", false, "The X position component"),
			prop("Y", "number", false, "The Y position component"),
			prop("Z", "number", false, "The Z position component"),
			prop("Position", "Vector3", false, "The position component as a Vector3"),
			prop("Rotation", "Vector3", false, "The rotation expressed as Euler angles (radians)"),
			prop("LookVector", "Vector3", false, "The forward direction (unit vector)"),
			prop("RightVector", "Vector3", false, "The right direction (unit vector)"),
			prop("UpVector", "Vector3", false, "The up direction (unit vector)"),
			prop("identity", "CFrame", false, "The identity CFrame (position at origin, no rotation)"),
		],
		methods: [
			method("Inverse", [], "CFrame", "Returns the inverse of this CFrame"),
			method("Lerp", [p("goal", "CFrame"), p("alpha", "number")], "CFrame", "Linearly interpolates position and rotation"),
			method("ToWorldSpace", [p("cf", "CFrame")], "CFrame", "Converts a CFrame from local to world space"),
			method("ToObjectSpace", [p("cf", "CFrame")], "CFrame", "Converts a CFrame from world to local space"),
			method("PointToWorldSpace", [p("v", "Vector3")], "Vector3", "Transforms a point from local to world space"),
			method("PointToObjectSpace", [p("v", "Vector3")], "Vector3", "Transforms a point from world to local space"),
			method("VectorToWorldSpace", [p("v", "Vector3")], "Vector3", "Transforms a direction from local to world space"),
			method("VectorToObjectSpace", [p("v", "Vector3")], "Vector3", "Transforms a direction from world to local space"),
			method("FuzzyEq", [p("other", "CFrame"), p("epsilon", "number")], "boolean", "Checks approximate equality"),
			method("Rotation", [], "Vector3", "Returns a copy with zero position"),
		],
		staticMethods: [
			statMethod("new", [p("x", "number"), p("y", "number"), p("z", "number")], "CFrame", "Creates a CFrame at a position with no rotation"),
			statMethod("new", [p("pos", "Vector3")], "CFrame", "Creates a CFrame at a position with no rotation"),
			statMethod("new", [p("x", "number"), p("y", "number"), p("z", "number"), p("qx", "number"), p("qy", "number"), p("qz", "number"), p("qw", "number")], "CFrame", "Creates a CFrame at position with quaternion rotation"),
			statMethod("lookAt", [p("eye", "Vector3"), p("target", "Vector3"), p("up", "Vector3")], "CFrame", "Creates a CFrame at 'eye' looking at 'target'"),
			statMethod("Angles", [p("rx", "number"), p("ry", "number"), p("rz", "number")], "CFrame", "Creates a CFrame from Euler angles (radians)"),
			statMethod("fromMatrix", [p("pos", "Vector3"), p("vx", "Vector3"), p("vy", "Vector3"), p("vz", "Vector3")], "CFrame", "Creates a CFrame from a position and basis vectors"),
			statMethod("fromEulerAnglesXYZ", [p("x", "number"), p("y", "number"), p("z", "number")], "CFrame", "Creates a CFrame from XYZ Euler angles (radians)"),
			statMethod("fromEulerAnglesYXZ", [p("y", "number"), p("x", "number"), p("z", "number")], "CFrame", "Creates a CFrame from YXZ Euler angles (radians)"),
			statMethod("fromAxisAngle", [p("axis", "Vector3"), p("angle", "number")], "CFrame", "Creates a CFrame rotated around an axis by given angle"),
			statMethod("fromOrientation", [p("rx", "number"), p("ry", "number"), p("rz", "number")], "CFrame", "Creates a CFrame at origin with given Euler orientation (radians)"),
		],
	},

	// ── Color3 ─────────────────────────────────────────────────────────────
	{
		name: "Color3",
		description: "Represents an RGB color with components from 0 to 1.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("R", "number", false, "R channel (0-1)"),
			prop("G", "number", false, "G channel (0-1)"),
			prop("B", "number", false, "B channel (0-1)"),
		],
		methods: [
			method("Lerp", [p("goal", "Color3"), p("alpha", "number")], "Color3", "Linearly interpolates between colors"),
			method("ToHSV", [], "tuple", "Converts to HSV (hue, saturation, value)"),
		],
		staticMethods: [
			statMethod("new", [p("r", "number"), p("g", "number"), p("b", "number")], "Color3", "Creates a Color3 from RGB (0-1)"),
			statMethod("fromRGB", [p("r", "number"), p("g", "number"), p("b", "number")], "Color3", "Creates a Color3 from 0-255 integer values"),
			statMethod("fromHSV", [p("h", "number"), p("s", "number"), p("v", "number")], "Color3", "Creates a Color3 from HSV"),
			statMethod("fromHex", [p("hex", "string")], "Color3", "Creates a Color3 from a hex string (e.g. '#FF0000')"),
		],
	},

	// ── UDim2 ──────────────────────────────────────────────────────────────
	{
		name: "UDim2",
		description: "A 2D UI dimension with scale and offset for each axis. Immutable value type.",
		extends: null,
		tags: ["ui", "builtin"],
		properties: [
			prop("X", "UDim", false, "The X dimension"),
			prop("Y", "UDim", false, "The Y dimension"),
			prop("Width", "UDim", false, "Alias for X"),
			prop("Height", "UDim", false, "Alias for Y"),
		],
		methods: [
			method("Lerp", [p("goal", "UDim2"), p("alpha", "number")], "UDim2", "Linearly interpolates between UDim2 values"),
		],
		staticMethods: [
			statMethod("new", [p("xScale", "number"), p("xOffset", "number"), p("yScale", "number"), p("yOffset", "number")], "UDim2", "Creates a new UDim2"),
			statMethod("fromScale", [p("x", "number"), p("y", "number")], "UDim2", "Creates a UDim2 from scale values (offsets = 0)"),
			statMethod("fromOffset", [p("x", "number"), p("y", "number")], "UDim2", "Creates a UDim2 from offset values (scales = 0)"),
		],
	},

	// ── UDim ───────────────────────────────────────────────────────────────
	{
		name: "UDim",
		description: "A 1D UI dimension with scale and offset. Immutable value type.",
		extends: null,
		tags: ["ui", "builtin"],
		properties: [
			prop("Scale", "number", false, "The scale component"),
			prop("Offset", "number", false, "The offset component"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("scale", "number"), p("offset", "number")], "UDim", "Creates a new UDim"),
			statMethod("fromScale", [p("scale", "number")], "UDim", "Creates a UDim from a scale value"),
			statMethod("fromOffset", [p("offset", "number")], "UDim", "Creates a UDim from an offset value"),
		],
	},

	// ── BrickColor ────────────────────────────────────────────────────────
	{
		name: "BrickColor",
		description: "Represents a Roblox BrickColor with a name, number, and Color3.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Number", "number", false, "The BrickColor ID"),
			prop("Name", "string", false, "The BrickColor name"),
			prop("Color", "Color3", false, "The Color3 representation"),
			prop("r", "number", false, "Red component (0-1)"),
			prop("g", "number", false, "Green component (0-1)"),
			prop("b", "number", false, "Blue component (0-1)"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("v", "number|string")], "BrickColor", "Creates a BrickColor from an ID or name"),
			statMethod("Random", [], "BrickColor", "Returns a random BrickColor"),
			statMethod("palette", [p("index", "number")], "BrickColor", "Returns the BrickColor at the given palette index"),
		],
	},

	// ── TweenInfo ─────────────────────────────────────────────────────────
	{
		name: "TweenInfo",
		description: "Configuration for a Tween animation.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Time", "number", false, "Duration of the tween in seconds"),
			prop("EasingStyle", "Enum.EasingStyle", false, "The easing style"),
			prop("EasingDirection", "Enum.EasingDirection", false, "The easing direction"),
			prop("RepeatCount", "number", false, "Number of times to repeat"),
			prop("Reverses", "boolean", false, "Whether the tween reverses on repeat"),
			prop("DelayTime", "number", false, "Delay before the tween starts"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [
				p("time", "number"),
				p("easingStyle", "Enum.EasingStyle"),
				p("easingDirection", "Enum.EasingDirection"),
				p("repeatCount", "number"),
				p("reverses", "boolean"),
				p("delayTime", "number"),
			], "TweenInfo", "Creates a new TweenInfo"),
		],
	},

	// ── Ray ───────────────────────────────────────────────────────────────
	{
		name: "Ray",
		description: "A ray defined by an origin and a direction.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("Origin", "Vector3", true, "The starting point of the ray"),
			prop("Direction", "Vector3", true, "The direction vector of the ray"),
			prop("Unit", "Ray", false, "A copy with a normalized direction"),
		],
		methods: [
			method("ClosestPoint", [p("point", "Vector3")], "Vector3", "Returns the closest point on the ray to the given point"),
			method("Distance", [p("point", "Vector3")], "number", "Returns the distance from the ray to the point"),
		],
		staticMethods: [
			statMethod("new", [p("origin", "Vector3"), p("direction", "Vector3")], "Ray", "Creates a new Ray"),
		],
	},

	// ── Region3 ───────────────────────────────────────────────────────────
	{
		name: "Region3",
		description: "An axis-aligned bounding box defined by two corner points.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("CFrame", "CFrame", false, "The center CFrame of the region"),
			prop("Size", "Vector3", false, "The size of the region"),
		],
		methods: [
			method("ExpandToGrid", [p("resolution", "number")], "Region3", "Expands the region to align with a grid"),
		],
		staticMethods: [
			statMethod("new", [p("min", "Vector3"), p("max", "Vector3")], "Region3", "Creates a new Region3 from two corners"),
		],
	},

	// ── OverlapParams ─────────────────────────────────────────────────────
	{
		name: "OverlapParams",
		description: "Configuration for spatial overlap queries (workspace:GetPartBoundsInRadius, etc.)",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("FilterType", "Enum.RaycastFilterType", true, "How the FilterDescendantsInstances list is interpreted"),
			prop("FilterDescendantsInstances", "table", true, "Array of instances to include or exclude"),
			prop("MaxParts", "number", true, "Maximum number of parts to return"),
			prop("BruteForceSlow", "boolean", true, "Opt into slower but more accurate brute-force mode"),
			prop("RespectCanCollide", "boolean", true, "Whether to respect CanCollide on parts"),
			prop("CollisionGroup", "string", true, "Collision group filter"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [], "OverlapParams", "Creates new OverlapParams"),
		],
	},

	// ── RaycastParams ─────────────────────────────────────────────────────
	{
		name: "RaycastParams",
		description: "Configuration for raycast queries (workspace:Raycast, etc.)",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("FilterType", "Enum.RaycastFilterType", true, "How the FilterDescendantsInstances list is interpreted"),
			prop("FilterDescendantsInstances", "table", true, "Array of instances to include or exclude"),
			prop("IgnoreWater", "boolean", true, "Whether to ignore Terrain water"),
			prop("CollisionGroup", "string", true, "Collision group filter"),
			prop("BruteForceSlow", "boolean", true, "Opt into slower but more accurate brute-force mode"),
			prop("RespectCanCollide", "boolean", true, "Whether to respect CanCollide on parts"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [], "RaycastParams", "Creates new RaycastParams"),
		],
	},

	// ── RaycastResult ─────────────────────────────────────────────────────
	{
		name: "RaycastResult",
		description: "The result of a raycast operation.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Instance", "Instance", false, "The BasePart or Terrain cell hit"),
			prop("Position", "Vector3", false, "The world-space hit position"),
			prop("Distance", "number", false, "The distance from the ray origin to the hit"),
			prop("Material", "Enum.Material", false, "The material at the hit point"),
			prop("Normal", "Vector3", false, "The surface normal at the hit point"),
		],
		methods: [],
		staticMethods: [],
	},

	// ── NumberRange ──────────────────────────────────────────────────────
	{
		name: "NumberRange",
		description: "A range of numbers with a minimum and maximum.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [
			prop("Min", "number", false, "The minimum value"),
			prop("Max", "number", false, "The maximum value"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("min", "number"), p("max", "number")], "NumberRange", "Creates a new NumberRange"),
		],
	},

	// ── NumberSequence ───────────────────────────────────────────────────
	{
		name: "NumberSequence",
		description: "A sequence of keypoints used to animate numbers over time.",
		extends: null,
		tags: ["builtin"],
		properties: [],
		methods: [],
		staticMethods: [
			statMethod("new", [p("keypoints", "table")], "NumberSequence", "Creates a new NumberSequence from keypoints table"),
			statMethod("new", [p("value", "number")], "NumberSequence", "Creates a constant NumberSequence"),
			statMethod("new", [p("time0", "number"), p("value0", "number"), p("time1", "number"), p("value1", "number")], "NumberSequence", "Creates a linear NumberSequence"),
		],
	},

	// ── ColorSequence ────────────────────────────────────────────────────
	{
		name: "ColorSequence",
		description: "A sequence of color keypoints used to animate colors over time.",
		extends: null,
		tags: ["builtin"],
		properties: [],
		methods: [],
		staticMethods: [
			statMethod("new", [p("keypoints", "table")], "ColorSequence", "Creates a new ColorSequence from keypoints table"),
			statMethod("new", [p("color", "Color3")], "ColorSequence", "Creates a constant ColorSequence"),
		],
	},

	// ── PhysicalProperties ───────────────────────────────────────────────
	{
		name: "PhysicalProperties",
		description: "Physical material properties for a BasePart.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Density", "number", false, "The density of the material"),
			prop("Friction", "number", false, "The friction coefficient"),
			prop("Elasticity", "number", false, "The elasticity (bounciness)"),
			prop("FrictionWeight", "number", false, "The friction weight"),
			prop("ElasticityWeight", "number", false, "The elasticity weight"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("material", "Enum.Material")], "PhysicalProperties", "Creates from a material preset"),
			statMethod("new", [p("density", "number"), p("friction", "number"), p("elasticity", "number")], "PhysicalProperties", "Creates custom properties"),
		],
	},

	// ── Faces ───────────────────────────────────────────────────────────
	{
		name: "Faces",
		description: "A bitmask indicating which faces of a part are active.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Top", "boolean", false, "Top face"),
			prop("Bottom", "boolean", false, "Bottom face"),
			prop("Left", "boolean", false, "Left face"),
			prop("Right", "boolean", false, "Right face"),
			prop("Front", "boolean", false, "Front face"),
			prop("Back", "boolean", false, "Back face"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("top", "boolean"), p("bottom", "boolean"), p("left", "boolean"), p("right", "boolean"), p("front", "boolean"), p("back", "boolean")], "Faces", "Creates a new Faces bitmask"),
		],
	},

	// ── Axes ─────────────────────────────────────────────────────────────
	{
		name: "Axes",
		description: "A bitmask indicating which axes are active.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("X", "boolean", false, "X axis"),
			prop("Y", "boolean", false, "Y axis"),
			prop("Z", "boolean", false, "Z axis"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("x", "boolean"), p("y", "boolean"), p("z", "boolean")], "Axes", "Creates a new Axes bitmask"),
		],
	},

	// ── Rect ─────────────────────────────────────────────────────────────
	{
		name: "Rect",
		description: "A 2D rectangle defined by minimum and maximum Vector2 corners.",
		extends: null,
		tags: ["ui", "builtin"],
		properties: [
			prop("Min", "Vector2", false, "The minimum corner"),
			prop("Max", "Vector2", false, "The maximum corner"),
			prop("Width", "number", false, "The width of the rectangle"),
			prop("Height", "number", false, "The height of the rectangle"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [p("min", "Vector2"), p("max", "Vector2")], "Rect", "Creates a new Rect"),
			statMethod("new", [p("x0", "number"), p("y0", "number"), p("x1", "number"), p("y1", "number")], "Rect", "Creates a Rect from coordinates"),
		],
	},

	// ── Random ───────────────────────────────────────────────────────────
	{
		name: "Random",
		description: "A deterministic random number generator.",
		extends: null,
		tags: ["math", "builtin"],
		properties: [],
		methods: [
			method("NextNumber", [], "number", "Returns a random number in [0, 1)"),
			method("NextNumber", [p("max", "number")], "number", "Returns a random number in [0, max)"),
			method("NextNumber", [p("min", "number"), p("max", "number")], "number", "Returns a random number in [min, max)"),
			method("NextInteger", [p("min", "number"), p("max", "number")], "number", "Returns a random integer in [min, max]"),
			method("Clone", [], "Random", "Returns a copy of this generator"),
			method("Shuffle", [p("t", "table")], "table", "Shuffles a table in-place"),
		],
		staticMethods: [
			statMethod("new", [], "Random", "Creates a new Random with a random seed"),
			statMethod("new", [p("seed", "number")], "Random", "Creates a new Random with the given seed"),
		],
	},

	// ── DateTime ─────────────────────────────────────────────────────────
	{
		name: "DateTime",
		description: "Represents a point in UTC time.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("UnixTimestamp", "number", false, "The UTC Unix timestamp (seconds since epoch)"),
			prop("UnixTimestampMillis", "number", false, "The UTC Unix timestamp in milliseconds"),
		],
		methods: [
			method("FormatLocalTime", [p("format", "string"), p("locale", "string")], "string", "Formats as local time string"),
			method("FormatUniversalTime", [p("format", "string"), p("locale", "string")], "string", "Formats as UTC time string"),
		],
		staticMethods: [
			statMethod("now", [], "DateTime", "Returns the current UTC DateTime"),
			statMethod("fromUnixTimestamp", [p("timestamp", "number")], "DateTime", "Creates a DateTime from a Unix timestamp"),
			statMethod("fromLocalTime", [p("year", "number"), p("month", "number"), p("day", "number"), p("hour", "number"), p("minute", "number"), p("second", "number")], "DateTime", "Creates a DateTime from local time components"),
		],
	},

	// ── PathWaypoint ─────────────────────────────────────────────────────
	{
		name: "PathWaypoint",
		description: "A single point in a Path computed by PathfindingService.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Position", "Vector3", false, "The world-space position of the waypoint"),
			prop("Action", "Enum.PathWaypointAction", false, "The action at this waypoint (Walk, Jump)"),
			prop("Label", "string", false, "A label for this waypoint"),
		],
		methods: [],
		staticMethods: [],
	},

	// ── DockWidgetPluginGuiInfo ─────────────────────────────────────────

	{
		name: "DockWidgetPluginGuiInfo",
		description: "Configuration for a dockable plugin widget.",
		extends: null,
		tags: ["plugin", "builtin"],
		properties: [
			prop("InitialDockState", "Enum.InitialDockState", false, "Initial dock state"),
			prop("InitialEnabled", "boolean", false, "Start enabled?"),
			prop("InitialEnabledShouldOverrideRestore", "boolean", false, "Override saved state?"),
			prop("FloatingXSize", "number", false, "Floating width"),
			prop("FloatingYSize", "number", false, "Floating height"),
			prop("FloatingXPosition", "number", false, "Floating X position"),
			prop("FloatingYPosition", "number", false, "Floating Y position"),
			prop("MinWidth", "number", false, "Minimum widget width"),
			prop("MinHeight", "number", false, "Minimum widget height"),
		],
		methods: [],
		staticMethods: [
			statMethod("new", [
				p("initialDockState", "Enum.InitialDockState"),
				p("initialEnabled", "boolean"),
				p("initialEnabledShouldOverrideRestore", "boolean"),
				p("floatingXSize", "number"),
				p("floatingYSize", "number"),
				p("floatingXPosition", "number"),
				p("floatingYPosition", "number"),
			], "DockWidgetPluginGuiInfo", "Creates a new DockWidgetPluginGuiInfo"),
		],
	},

	// ── RBXScriptSignal ─────────────────────────────────────────────────
	{
		name: "RBXScriptSignal",
		description: "An event signal that can be connected to, waited on, and fired.",
		extends: null,
		tags: ["builtin"],
		properties: [],
		methods: [
			method("Wait", [], "any", "Yields until the event fires, then returns the arguments"),
			method("Connect", [p("callback", "function")], "RBXScriptConnection", "Connects a function to be called when the event fires"),
			method("Once", [p("callback", "function")], "void", "Connects a function that will only be called once"),
		],
		staticMethods: [],
	},

	// ── RBXScriptConnection ─────────────────────────────────────────────
	{
		name: "RBXScriptConnection",
		description: "A connection to an event signal, allowing disconnection.",
		extends: null,
		tags: ["builtin"],
		properties: [
			prop("Connected", "boolean", false, "Whether the connection is still active"),
		],
		methods: [
			method("Disconnect", [], "void", "Disconnects the event connection"),
		],
		staticMethods: [],
	},
];

// ── Globals that should be added to the bindings global map ───────────────

export const LUAU_GLOBALS = [
	"Vector3", "Vector2", "CFrame", "Color3", "BrickColor",
	"UDim2", "UDim", "TweenInfo", "Ray", "Region3",
	"OverlapParams", "RaycastParams", "RaycastResult",
	"NumberRange", "NumberSequence", "ColorSequence",
	"PhysicalProperties", "Faces", "Axes", "Rect",
	"Random", "DateTime", "PathWaypoint", "DockWidgetPluginGuiInfo",
];

// ── Injection interface (duck-typed for both WoldBindings and LSP Bindings) ─

export interface DatatypeSink {
	types: Map<string, LuauDatatypeSink>;
	globals: Map<string, GlobalSink>;
	functions: Map<string, FunctionSink>;
}

interface LuauDatatypeSink {
	name: string;
	description: string;
	extends?: string | null;
	tags: string[];
	properties: { name: string; type: string; rw: boolean; description: string; }[];
	methods: { name: string; params: { name: string; type: string; }[]; returns: string; description: string; }[];
	events: any[];
}

interface GlobalSink {
	name: string;
	type: string;
	description: string;
}

interface FunctionSink {
	name: string;
	params: { name: string; type: string; }[];
	returns: string;
	description: string;
}

export function injectLuauDatatypes(sink: DatatypeSink): void {
	for (const dt of LUAU_DATATYPES) {
		const name = dt.name;
		sink.types.set(name.toLowerCase(), {
			name,
			description: dt.description,
			extends: dt.extends,
			tags: dt.tags,
			properties: dt.properties.map(p => ({ name: p.name, type: p.type, rw: p.rw, description: p.description })),
			methods: dt.methods.map(m => ({ name: m.name, params: m.params.map(pp => ({ name: pp.name, type: pp.type })), returns: m.returns, description: m.description })),
			events: [],
		});
		sink.globals.set(name.toLowerCase(), { name, type: name, description: dt.description });
		for (const sm of dt.staticMethods) {
			const funcName = `${name}.${sm.name}`;
			sink.functions.set(funcName.toLowerCase(), {
				name: funcName,
				params: sm.params.map(pp => ({ name: pp.name, type: pp.type })),
				returns: sm.returns,
				description: sm.description,
			});
		}
	}
}
