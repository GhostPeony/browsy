import type { z } from "zod";

/**
 * Minimal Zod-to-JSON-Schema converter for tool parameter schemas.
 * Handles the subset of Zod types used in browsy schemas (object, string, number, optional).
 */
export function zodToJsonSchema(schema: z.ZodTypeAny): Record<string, unknown> {
  return convertType(schema);
}

function convertType(schema: z.ZodTypeAny): Record<string, unknown> {
  const def = schema._def;

  // ZodObject
  if (def.typeName === "ZodObject") {
    const shape = (schema as z.ZodObject<z.ZodRawShape>).shape;
    const properties: Record<string, unknown> = {};
    const required: string[] = [];

    for (const [key, value] of Object.entries(shape)) {
      const fieldSchema = value as z.ZodTypeAny;
      const fieldDef = fieldSchema._def;

      if (fieldDef.typeName === "ZodOptional") {
        properties[key] = convertType(fieldDef.innerType);
        const desc = fieldSchema.description ?? fieldDef.innerType.description;
        if (desc) (properties[key] as Record<string, unknown>).description = desc;
      } else {
        properties[key] = convertType(fieldSchema);
        if (fieldSchema.description) {
          (properties[key] as Record<string, unknown>).description = fieldSchema.description;
        }
        required.push(key);
      }
    }

    const result: Record<string, unknown> = {
      type: "object",
      properties,
    };
    if (required.length > 0) {
      result.required = required;
    }
    return result;
  }

  // ZodString
  if (def.typeName === "ZodString") {
    return { type: "string" };
  }

  // ZodNumber
  if (def.typeName === "ZodNumber") {
    return { type: "number" };
  }

  // ZodOptional — unwrap
  if (def.typeName === "ZodOptional") {
    return convertType(def.innerType);
  }

  // Fallback
  return {};
}
