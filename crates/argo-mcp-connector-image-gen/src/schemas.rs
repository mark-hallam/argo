//! MCP tool schema for image-gen.

use argo_mcp::McpToolSchema;

/// All image-gen tool schemas.
pub fn all() -> Vec<McpToolSchema> {
    vec![generate_image()]
}

fn generate_image() -> McpToolSchema {
    McpToolSchema {
        name: "image-gen.generate_image".into(),
        description:
            "Generate one image from a text prompt. Writes a PNG to <workspace>/.argo/generated/ and returns a file:// URL. Approval-gated — image generation has cost and reputational implications."
                .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": true,
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Text description of the image to render."
                },
                "style": {
                    "type": "string",
                    "description": "Optional style hint passed through to the backend (e.g. 'vivid', 'natural', 'photographic')."
                },
                "size": {
                    "type": "string",
                    "description": "Optional size hint, '<width>x<height>' (e.g. '1024x1024'). Backends fall back to a sane default if unsupported."
                },
                "backend": {
                    "type": "string",
                    "enum": ["openai", "stability", "replicate"],
                    "description": "Override IMAGE_GEN_BACKEND env for one call."
                }
            },
            "required": ["prompt"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_marks_generate_image_approval_required() {
        let tools = all();
        assert_eq!(tools.len(), 1);
        let s = &tools[0];
        assert_eq!(s.name, "image-gen.generate_image");
        assert_eq!(s.input_schema["approval_required"], true);
        assert_eq!(s.input_schema["required"][0], "prompt");
    }
}
