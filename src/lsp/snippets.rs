use lsp_types::*;

pub fn get_snippets() -> Vec<CompletionItem> {
    vec![
        // ModuleScript boilerplate
        snippet(
            "mod-new",
            CompletionItemKind::SNIPPET,
            "ModuleScript boilerplate",
            "snippet",
            &[
                "local ${1:module} = {}",
                "",
                "function ${1:module}.${2:init}()",
                "    ${0}",
                "end",
                "",
                "return ${1:module}",
            ].join("\n"),
        ),
        // OOP class with metatable
        snippet(
            "class-oop",
            CompletionItemKind::SNIPPET,
            "OOP class with new(), __index, init()",
            "snippet",
            &[
                "local ${1:ClassName} = {}",
                "${1:ClassName}.__index = ${1:ClassName}",
                "",
                "function ${1:ClassName}.new()",
                "    local self = setmetatable({}, ${1:ClassName})",
                "    self:init()",
                "    return self",
                "end",
                "",
                "function ${1:ClassName}:init()",
                "    ${0}",
                "end",
            ].join("\n"),
        ),
        // Event connection
        snippet(
            "event-connect",
            CompletionItemKind::SNIPPET,
            "Event:Connect handler",
            "snippet",
            &[
                "${1:event}:Connect(function(${2:args})",
                "    ${0}",
                "end)",
            ].join("\n"),
        ),
        // GetService
        snippet(
            "GetService",
            CompletionItemKind::SNIPPET,
            "GetService with local binding",
            "snippet",
            "local ${2:Service} = game:GetService(\"${1:ServiceName}\")",
        ),
        // For loop with range
        snippet(
            "for-range",
            CompletionItemKind::SNIPPET,
            "Numeric for loop with range",
            "snippet",
            &[
                "for ${1:i} in range(${2:0}, ${3:10}) {",
                "    ${0}",
                "}",
            ].join("\n"),
        ),
        // Try-catch
        snippet(
            "try-catch",
            CompletionItemKind::SNIPPET,
            "Try/catch block",
            "snippet",
            &[
                "try {",
                "    ${0}",
                "} catch ${1:err} {",
                "    warn(${1:err})",
                "}",
            ].join("\n"),
        ),
        // Guard clause
        snippet(
            "if-guard",
            CompletionItemKind::SNIPPET,
            "Guard clause with early return",
            "snippet",
            &[
                "if not ${1:condition} {",
                "    return ${2:nil}",
                "}",
                "${0}",
            ].join("\n"),
        ),
    ]
}

fn snippet(label: &str, kind: CompletionItemKind, detail: &str, sort_prefix: &str, text: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        insert_text: Some(text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("{}{}", sort_prefix, label)),
        ..Default::default()
    }
}
