use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub prop_type: &'static str,
    pub read_only: bool,
    pub tags: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub params: Vec<(&'static str, &'static str)>,
    pub return_type: &'static str,
    pub tags: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ClassApi {
    pub properties: HashMap<&'static str, PropertyInfo>,
    pub methods: HashMap<&'static str, MethodInfo>,
    pub super_class: Option<&'static str>,
}

pub struct RobloxApi {
    classes: HashMap<&'static str, ClassApi>,
    deprecated: HashMap<&'static str, &'static str>,
}

impl RobloxApi {
    pub fn new() -> Self {
        let mut classes = HashMap::new();

        // Instance (base class)
        {
            let mut props = HashMap::new();
            props.insert("Name", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            props.insert("Parent", PropertyInfo { prop_type: "Instance", read_only: false, tags: vec![] });
            props.insert("ClassName", PropertyInfo { prop_type: "string", read_only: true, tags: vec![] });
            props.insert("Archivable", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("Destroy", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("Clone", MethodInfo { params: vec![], return_type: "Instance", tags: vec![] });
            methods.insert("FindFirstChild", MethodInfo { params: vec![("name", "string"), ("recursive", "boolean")], return_type: "Instance?", tags: vec![] });
            methods.insert("FindFirstChildOfClass", MethodInfo { params: vec![("className", "string")], return_type: "Instance?", tags: vec![] });
            methods.insert("FindFirstChildWhichIsA", MethodInfo { params: vec![("className", "string")], return_type: "Instance?", tags: vec![] });
            methods.insert("GetChildren", MethodInfo { params: vec![], return_type: "Array<Instance>", tags: vec![] });
            methods.insert("GetDescendants", MethodInfo { params: vec![], return_type: "Array<Instance>", tags: vec![] });
            methods.insert("IsA", MethodInfo { params: vec![("className", "string")], return_type: "boolean", tags: vec![] });
            methods.insert("IsDescendantOf", MethodInfo { params: vec![("ancestor", "Instance")], return_type: "boolean", tags: vec![] });
            methods.insert("GetFullName", MethodInfo { params: vec![], return_type: "string", tags: vec![] });
            methods.insert("FindFirstAncestor", MethodInfo { params: vec![("name", "string")], return_type: "Instance?", tags: vec![] });
            methods.insert("FindFirstAncestorOfClass", MethodInfo { params: vec![("className", "string")], return_type: "Instance?", tags: vec![] });
            methods.insert("WaitForChild", MethodInfo { params: vec![("name", "string"), ("timeout", "number")], return_type: "Instance", tags: vec![] });
            classes.insert("Instance", ClassApi { properties: props, methods, super_class: None });
        }

        // BasePart > Instance
        {
            let mut props = HashMap::new();
            props.insert("Position", PropertyInfo { prop_type: "Vector3", read_only: false, tags: vec![] });
            props.insert("Size", PropertyInfo { prop_type: "Vector3", read_only: false, tags: vec![] });
            props.insert("Orientation", PropertyInfo { prop_type: "Vector3", read_only: false, tags: vec![] });
            props.insert("CFrame", PropertyInfo { prop_type: "CFrame", read_only: false, tags: vec![] });
            props.insert("Anchored", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("CanCollide", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("Transparency", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("Color", PropertyInfo { prop_type: "Color3", read_only: false, tags: vec![] });
            props.insert("BrickColor", PropertyInfo { prop_type: "BrickColor", read_only: false, tags: vec![] });
            props.insert("Material", PropertyInfo { prop_type: "Material", read_only: false, tags: vec![] });
            props.insert("Locked", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("Reflectance", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("Mass", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("BreakJoints", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("MakeJoints", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("GetMass", MethodInfo { params: vec![], return_type: "number", tags: vec![] });
            methods.insert("GetTouchingParts", MethodInfo { params: vec![], return_type: "Array<BasePart>", tags: vec![] });
            methods.insert("Touched", MethodInfo { params: vec![("otherPart", "BasePart")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            methods.insert("TouchEnded", MethodInfo { params: vec![("otherPart", "BasePart")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("BasePart", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }
        // Part > BasePart
        {
            let props = HashMap::new();
            let methods = HashMap::new();
            classes.insert("Part", ClassApi { properties: props, methods, super_class: Some("BasePart") });
        }

        // Model > Instance
        {
            let mut props = HashMap::new();
            props.insert("PrimaryPart", PropertyInfo { prop_type: "BasePart?", read_only: false, tags: vec![] });
            props.insert("WorldPivot", PropertyInfo { prop_type: "CFrame", read_only: false, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("MoveTo", MethodInfo { params: vec![("position", "Vector3")], return_type: "nil", tags: vec![] });
            methods.insert("SetPrimaryPartCFrame", MethodInfo { params: vec![("cframe", "CFrame")], return_type: "nil", tags: vec![] });
            methods.insert("GetBoundingBox", MethodInfo { params: vec![], return_type: "CFrame,Vector3", tags: vec![] });
            methods.insert("GetExtentsSize", MethodInfo { params: vec![], return_type: "Vector3", tags: vec![] });
            methods.insert("GetPivot", MethodInfo { params: vec![], return_type: "CFrame", tags: vec![] });
            methods.insert("PivotTo", MethodInfo { params: vec![("cframe", "CFrame")], return_type: "nil", tags: vec![] });
            classes.insert("Model", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // Player > Instance
        {
            let mut props = HashMap::new();
            props.insert("Character", PropertyInfo { prop_type: "Model?", read_only: false, tags: vec![] });
            props.insert("UserId", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] });
            props.insert("DisplayName", PropertyInfo { prop_type: "string", read_only: true, tags: vec![] });
            props.insert("Team", PropertyInfo { prop_type: "Team?", read_only: false, tags: vec![] });
            props.insert("Backpack", PropertyInfo { prop_type: "Tool", read_only: true, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("Kick", MethodInfo { params: vec![("message", "string")], return_type: "nil", tags: vec![] });
            methods.insert("LoadCharacter", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("GetPlayerFromCharacter", MethodInfo { params: vec![("character", "Model")], return_type: "Player?", tags: vec![] });
            classes.insert("Player", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // Humanoid > Instance
        {
            let mut props = HashMap::new();
            props.insert("Health", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("MaxHealth", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("WalkSpeed", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("JumpPower", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("HipHeight", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("Sit", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("RigType", PropertyInfo { prop_type: "HumanoidRigType", read_only: true, tags: vec![] });
            props.insert("SeatPart", PropertyInfo { prop_type: "BasePart?", read_only: false, tags: vec![] });
            props.insert("MoveDirection", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("TakeDamage", MethodInfo { params: vec![("damage", "number")], return_type: "nil", tags: vec![] });
            methods.insert("Move", MethodInfo { params: vec![("moveDirection", "Vector3"), ("relativeToCamera", "boolean")], return_type: "nil", tags: vec![] });
            methods.insert("MoveTo", MethodInfo { params: vec![("location", "Vector3"), ("part", "BasePart")], return_type: "nil", tags: vec![] });
            methods.insert("Jump", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("GetState", MethodInfo { params: vec![], return_type: "HumanoidStateType", tags: vec![] });
            methods.insert("EquipTool", MethodInfo { params: vec![("tool", "Tool")], return_type: "nil", tags: vec![] });
            methods.insert("UnequipTools", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("Died", MethodInfo { params: vec![], return_type: "RBXScriptSignal", tags: vec!["event"] });
            methods.insert("Running", MethodInfo { params: vec![("speed", "number")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("Humanoid", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // Players > Instance
        {
            let mut props = HashMap::new();
            props.insert("LocalPlayer", PropertyInfo { prop_type: "Player", read_only: true, tags: vec!["client-only"] });
            props.insert("RespawnTime", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("MaxPlayers", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("GetPlayers", MethodInfo { params: vec![], return_type: "Array<Player>", tags: vec![] });
            methods.insert("GetPlayerByUserId", MethodInfo { params: vec![("userId", "number")], return_type: "Player?", tags: vec![] });
            methods.insert("GetPlayerFromCharacter", MethodInfo { params: vec![("character", "Model")], return_type: "Player?", tags: vec![] });
            methods.insert("PlayerAdded", MethodInfo { params: vec![("player", "Player")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            methods.insert("PlayerRemoving", MethodInfo { params: vec![("player", "Player")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("Players", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // DataModel (game) > Instance
        {
            let mut methods = HashMap::new();
            methods.insert("GetService", MethodInfo { params: vec![("service", "string")], return_type: "Instance", tags: vec![] });
            methods.insert("IsLoaded", MethodInfo { params: vec![], return_type: "boolean", tags: vec![] });
            classes.insert("DataModel", ClassApi { properties: HashMap::new(), methods, super_class: Some("Instance") });
        }

        // Script (BaseScript) > Instance
        {
            let mut props = HashMap::new();
            props.insert("Enabled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("Source", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            classes.insert("Script", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }
        {
            let mut props = HashMap::new();
            props.insert("Enabled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            classes.insert("LocalScript", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }
        {
            let mut props = HashMap::new();
            props.insert("Enabled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            classes.insert("ModuleScript", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }

        // ScreenGui > Instance
        {
            let mut props = HashMap::new();
            props.insert("Enabled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("ResetOnSpawn", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("ZIndexBehavior", PropertyInfo { prop_type: "ZIndexBehavior", read_only: false, tags: vec![] });
            classes.insert("ScreenGui", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }

        // TextLabel > Instance
        {
            let mut props = HashMap::new();
            props.insert("Text", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            props.insert("TextSize", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("TextColor3", PropertyInfo { prop_type: "Color3", read_only: false, tags: vec![] });
            props.insert("Font", PropertyInfo { prop_type: "Font", read_only: false, tags: vec![] });
            props.insert("TextScaled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("TextWrapped", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("TextXAlignment", PropertyInfo { prop_type: "TextXAlignment", read_only: false, tags: vec![] });
            props.insert("TextYAlignment", PropertyInfo { prop_type: "TextYAlignment", read_only: false, tags: vec![] });
            props.insert("TextStrokeTransparency", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            classes.insert("TextLabel", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }

        // TextButton > TextLabel
        {
            let mut methods = HashMap::new();
            methods.insert("Activated", MethodInfo { params: vec![], return_type: "RBXScriptSignal", tags: vec!["event"] });
            methods.insert("MouseButton1Click", MethodInfo { params: vec![], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("TextButton", ClassApi { properties: HashMap::new(), methods, super_class: Some("TextLabel") });
        }
        // Frame > Instance
        {
            let mut props = HashMap::new();
            props.insert("BackgroundColor3", PropertyInfo { prop_type: "Color3", read_only: false, tags: vec![] });
            props.insert("BackgroundTransparency", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("Size", PropertyInfo { prop_type: "UDim2", read_only: false, tags: vec![] });
            props.insert("Position", PropertyInfo { prop_type: "UDim2", read_only: false, tags: vec![] });
            props.insert("AnchorPoint", PropertyInfo { prop_type: "Vector2", read_only: false, tags: vec![] });
            props.insert("ClipsDescendants", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            classes.insert("Frame", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }

        // Sound > Instance
        {
            let mut props = HashMap::new();
            props.insert("SoundId", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            props.insert("Volume", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("PlaybackSpeed", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("Looped", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("TimePosition", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] });
            props.insert("IsPlaying", PropertyInfo { prop_type: "boolean", read_only: true, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("Play", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("Stop", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("Pause", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            methods.insert("Resume", MethodInfo { params: vec![], return_type: "nil", tags: vec![] });
            classes.insert("Sound", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // RemoteEvent > Instance
        {
            let mut methods = HashMap::new();
            methods.insert("FireServer", MethodInfo { params: vec![("args", "...")], return_type: "nil", tags: vec!["server-only"] });
            methods.insert("FireClient", MethodInfo { params: vec![("player", "Player"), ("args", "...")], return_type: "nil", tags: vec![] });
            methods.insert("FireAllClients", MethodInfo { params: vec![("args", "...")], return_type: "nil", tags: vec![] });
            methods.insert("OnServerEvent", MethodInfo { params: vec![("player", "Player"), ("args", "...")], return_type: "RBXScriptSignal", tags: vec!["event", "server-only"] });
            methods.insert("OnClientEvent", MethodInfo { params: vec![("args", "...")], return_type: "RBXScriptSignal", tags: vec!["event", "client-only"] });
            classes.insert("RemoteEvent", ClassApi { properties: HashMap::new(), methods, super_class: Some("Instance") });
        }

        // RemoteFunction > Instance
        {
            let mut methods = HashMap::new();
            methods.insert("InvokeServer", MethodInfo { params: vec![("args", "...")], return_type: "Tuple", tags: vec![] });
            methods.insert("InvokeClient", MethodInfo { params: vec![("player", "Player"), ("args", "...")], return_type: "Tuple", tags: vec![] });
            methods.insert("OnServerInvoke", MethodInfo { params: vec![("player", "Player"), ("args", "...")], return_type: "RBXScriptSignal", tags: vec!["callback", "server-only"] });
            methods.insert("OnClientInvoke", MethodInfo { params: vec![("args", "...")], return_type: "RBXScriptSignal", tags: vec!["callback", "client-only"] });
            classes.insert("RemoteFunction", ClassApi { properties: HashMap::new(), methods, super_class: Some("Instance") });
        }

        // BindableEvent > Instance
        {
            let mut methods = HashMap::new();
            methods.insert("Fire", MethodInfo { params: vec![("args", "...")], return_type: "nil", tags: vec![] });
            methods.insert("Event", MethodInfo { params: vec![("args", "...")], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("BindableEvent", ClassApi { properties: HashMap::new(), methods, super_class: Some("Instance") });
        }

        // Tool > Instance
        {
            let mut props = HashMap::new();
            props.insert("Enabled", PropertyInfo { prop_type: "boolean", read_only: false, tags: vec![] });
            props.insert("ToolTip", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            let mut methods = HashMap::new();
            methods.insert("Activated", MethodInfo { params: vec![], return_type: "RBXScriptSignal", tags: vec!["event"] });
            methods.insert("Deactivated", MethodInfo { params: vec![], return_type: "RBXScriptSignal", tags: vec!["event"] });
            classes.insert("Tool", ClassApi { properties: props, methods, super_class: Some("Instance") });
        }

        // Animation > Instance
        {
            let mut props = HashMap::new();
            props.insert("AnimationId", PropertyInfo { prop_type: "string", read_only: false, tags: vec![] });
            classes.insert("Animation", ClassApi { properties: props, methods: HashMap::new(), super_class: Some("Instance") });
        }

        // Animator > Instance
        {
            let mut methods = HashMap::new();
            methods.insert("LoadAnimation", MethodInfo { params: vec![("animation", "Animation")], return_type: "AnimationTrack", tags: vec![] });
            methods.insert("GetPlayingAnimationTracks", MethodInfo { params: vec![], return_type: "Array<AnimationTrack>", tags: vec![] });
            classes.insert("Animator", ClassApi { properties: HashMap::new(), methods, super_class: Some("Instance") });
        }

        // ValueBase subclasses
        {
            let mut props = HashMap::new();
            props.insert("Value", PropertyInfo { prop_type: "any", read_only: false, tags: vec![] });
            classes.insert("StringValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("IntValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("NumberValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("BoolValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("ObjectValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("CFrameValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("Vector3Value", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("Color3Value", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
            classes.insert("BrickColorValue", ClassApi { properties: props.clone(), methods: HashMap::new(), super_class: Some("Instance") });
        }

        // Data types (no super_class since they're not Instances)
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("x", "number"), ("y", "number"), ("z", "number")], return_type: "Vector3", tags: vec![] });
            methods.insert("Dot", MethodInfo { params: vec![("other", "Vector3")], return_type: "number", tags: vec![] });
            methods.insert("Cross", MethodInfo { params: vec![("other", "Vector3")], return_type: "Vector3", tags: vec![] });
            methods.insert("Magnitude", MethodInfo { params: vec![], return_type: "number", tags: vec![] });
            methods.insert("Unit", MethodInfo { params: vec![], return_type: "Vector3", tags: vec![] });
            methods.insert("Abs", MethodInfo { params: vec![], return_type: "Vector3", tags: vec![] });
            methods.insert("Lerp", MethodInfo { params: vec![("goal", "Vector3"), ("alpha", "number")], return_type: "Vector3", tags: vec![] });
            let props = [
                ("X", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] }),
                ("Y", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] }),
                ("Z", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] }),
            ].into_iter().collect();
            classes.insert("Vector3", ClassApi { properties: props, methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("x", "number"), ("y", "number")], return_type: "Vector2", tags: vec![] });
            methods.insert("Dot", MethodInfo { params: vec![("other", "Vector2")], return_type: "number", tags: vec![] });
            methods.insert("Magnitude", MethodInfo { params: vec![], return_type: "number", tags: vec![] });
            methods.insert("Unit", MethodInfo { params: vec![], return_type: "Vector2", tags: vec![] });
            methods.insert("Lerp", MethodInfo { params: vec![("goal", "Vector2"), ("alpha", "number")], return_type: "Vector2", tags: vec![] });
            let props = [
                ("X", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] }),
                ("Y", PropertyInfo { prop_type: "number", read_only: false, tags: vec![] }),
            ].into_iter().collect();
            classes.insert("Vector2", ClassApi { properties: props, methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("x", "number"), ("y", "number"), ("z", "number")], return_type: "CFrame", tags: vec![] });
            methods.insert("Angles", MethodInfo { params: vec![("rx", "number"), ("ry", "number"), ("rz", "number")], return_type: "CFrame", tags: vec![] });
            methods.insert("lookAt", MethodInfo { params: vec![("at", "Vector3"), ("pos", "Vector3")], return_type: "CFrame", tags: vec![] });
            methods.insert("Lerp", MethodInfo { params: vec![("goal", "CFrame"), ("alpha", "number")], return_type: "CFrame", tags: vec![] });
            methods.insert("Inverse", MethodInfo { params: vec![], return_type: "CFrame", tags: vec![] });
            methods.insert("ToWorldSpace", MethodInfo { params: vec![("cf", "CFrame")], return_type: "CFrame", tags: vec![] });
            methods.insert("ToObjectSpace", MethodInfo { params: vec![("cf", "CFrame")], return_type: "CFrame", tags: vec![] });
            let props = [
                ("Position", PropertyInfo { prop_type: "Vector3", read_only: false, tags: vec![] }),
                ("LookVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("RightVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("UpVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("XVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("YVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("ZVector", PropertyInfo { prop_type: "Vector3", read_only: true, tags: vec![] }),
                ("X", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
                ("Y", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
                ("Z", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
            ].into_iter().collect();
            classes.insert("CFrame", ClassApi { properties: props, methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("xScale", "number"), ("xOffset", "number"), ("yScale", "number"), ("yOffset", "number")], return_type: "UDim2", tags: vec![] });
            let props = [
                ("X", PropertyInfo { prop_type: "UDim", read_only: false, tags: vec![] }),
                ("Y", PropertyInfo { prop_type: "UDim", read_only: false, tags: vec![] }),
            ].into_iter().collect();
            classes.insert("UDim2", ClassApi { properties: props, methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("scale", "number"), ("offset", "number")], return_type: "UDim", tags: vec![] });
            classes.insert("UDim", ClassApi { properties: HashMap::new(), methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("r", "number"), ("g", "number"), ("b", "number")], return_type: "Color3", tags: vec![] });
            methods.insert("fromRGB", MethodInfo { params: vec![("r", "number"), ("g", "number"), ("b", "number")], return_type: "Color3", tags: vec![] });
            methods.insert("fromHSV", MethodInfo { params: vec![("h", "number"), ("s", "number"), ("v", "number")], return_type: "Color3", tags: vec![] });
            methods.insert("Lerp", MethodInfo { params: vec![("goal", "Color3"), ("alpha", "number")], return_type: "Color3", tags: vec![] });
            let props: HashMap<&str, PropertyInfo> = [
                ("R", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
                ("G", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
                ("B", PropertyInfo { prop_type: "number", read_only: true, tags: vec![] }),
            ].into_iter().collect();
            classes.insert("Color3", ClassApi { properties: props, methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("name", "string")], return_type: "BrickColor", tags: vec![] });
            methods.insert("Random", MethodInfo { params: vec![], return_type: "BrickColor", tags: vec![] });
            classes.insert("BrickColor", ClassApi { properties: HashMap::new(), methods, super_class: None });
        }
        {
            let mut methods = HashMap::new();
            methods.insert("new", MethodInfo { params: vec![("time", "number"), ("easingStyle", "Enum"), ("easingDirection", "Enum"), ("repeatCount", "number"), ("reverses", "boolean"), ("delayTime", "number")], return_type: "TweenInfo", tags: vec![] });
            classes.insert("TweenInfo", ClassApi { properties: HashMap::new(), methods, super_class: None });
        }

        // Deprecated globals with suggested replacements
        let mut deprecated = HashMap::new();
        deprecated.insert("wait", "task.wait");
        deprecated.insert("spawn", "task.spawn");
        deprecated.insert("delay", "task.delay");
        deprecated.insert("tick", "os.clock");
        deprecated.insert("ElapsedTime", "workspace:GetServerTimeNow()");
        deprecated.insert("time", "os.time");
        deprecated.insert("LoadLibrary", "require");
        deprecated.insert("settings", ""); // removed entirely

        RobloxApi { classes, deprecated }
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassApi> {
        self.classes.get(name)
    }

    pub fn has_property(&self, class_name: &str, prop: &str) -> bool {
        self.lookup_class(class_name)
            .map(|c| c.properties.contains_key(prop))
            .unwrap_or(false)
    }

    pub fn property_info(&self, class_name: &str, prop: &str) -> Option<&PropertyInfo> {
        self.lookup_class(class_name)
            .and_then(|c| c.properties.get(prop))
    }

    pub fn has_method(&self, class_name: &str, method: &str) -> bool {
        self.lookup_class(class_name)
            .map(|c| c.methods.contains_key(method))
            .unwrap_or(false)
    }

    pub fn method_info(&self, class_name: &str, method: &str) -> Option<&MethodInfo> {
        self.lookup_class(class_name)
            .and_then(|c| c.methods.get(method))
    }

    pub fn get_deprecation(&self, name: &str) -> Option<&str> {
        self.deprecated.get(name).copied()
    }

    pub fn is_known_class(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    fn lookup_class(&self, name: &str) -> Option<&ClassApi> {
        if let Some(cls) = self.classes.get(name) {
            return Some(cls);
        }
        None
    }

    fn lookup_class_property(&self, class_name: &str, prop: &str) -> Option<&PropertyInfo> {
        let mut current = class_name;
        loop {
            if let Some(cls) = self.classes.get(current) {
                if let Some(info) = cls.properties.get(prop) {
                    return Some(info);
                }
                if let Some(parent) = cls.super_class {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    fn lookup_class_method(&self, class_name: &str, method: &str) -> Option<&MethodInfo> {
        let mut current = class_name;
        loop {
            if let Some(cls) = self.classes.get(current) {
                if let Some(info) = cls.methods.get(method) {
                    return Some(info);
                }
                if let Some(parent) = cls.super_class {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }

    pub fn check_property(&self, class_name: &str, prop: &str) -> Option<&PropertyInfo> {
        self.lookup_class_property(class_name, prop)
    }

    pub fn check_method(&self, class_name: &str, method: &str) -> Option<&MethodInfo> {
        self.lookup_class_method(class_name, method)
    }
}
