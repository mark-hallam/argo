"use client";

import { useForm, type DefaultValues, type Resolver, type UseFormProps, type UseFormReturn } from "react-hook-form";
import type { z, ZodTypeAny } from "zod";

/**
 * Thin wrapper around `react-hook-form` + Zod. Centralises validation
 * so every form gets: typed schema, inline `formState.errors`, a
 * stable `formState.isSubmitting` for disable-while-submitting, and
 * `formState.isDirty` for save-button gating.
 *
 * `@hookform/resolvers/zod` is not in package.json — we ship a tiny
 * resolver below to avoid a new dep. It supports nested paths
 * (`a.b.c`) and array indices, which is all our forms need.
 *
 * Usage:
 *   const form = useZodForm(channelsSchema, { defaultValues });
 *   const onSubmit = form.handleSubmit(async (values) => { ... });
 */

export type UseZodFormOptions<TSchema extends ZodTypeAny> = Omit<
  UseFormProps<z.input<TSchema>>,
  "resolver"
> & {
  defaultValues: DefaultValues<z.input<TSchema>>;
};

export function useZodForm<TSchema extends ZodTypeAny>(
  schema: TSchema,
  options: UseZodFormOptions<TSchema>,
): UseFormReturn<z.input<TSchema>, unknown, z.output<TSchema>> {
  return useForm<z.input<TSchema>, unknown, z.output<TSchema>>({
    ...options,
    resolver: zodResolver(schema),
    mode: options.mode ?? "onSubmit",
  });
}

/**
 * Minimal Zod → react-hook-form resolver. Converts Zod issues into
 * RHF's nested-error shape so `formState.errors.foo.bar` works for
 * schemas with object/array sub-paths.
 */
export function zodResolver<TSchema extends ZodTypeAny>(
  schema: TSchema,
): Resolver<z.input<TSchema>, unknown, z.output<TSchema>> {
  return async (values) => {
    const result = schema.safeParse(values);
    if (result.success) {
      return { values: result.data as z.output<TSchema>, errors: {} };
    }
    const errors: Record<string, unknown> = {};
    for (const issue of result.error.issues) {
      const path = issue.path.length === 0 ? ["root"] : issue.path;
      assignErrorAtPath(errors, path, {
        type: issue.code,
        message: issue.message,
      });
    }
    return { values: {}, errors: errors as never };
  };
}

function assignErrorAtPath(
  target: Record<string, unknown>,
  path: ReadonlyArray<string | number>,
  error: { type: string; message: string },
): void {
  let cursor: Record<string, unknown> = target;
  for (let i = 0; i < path.length - 1; i += 1) {
    const segment = String(path[i]);
    const existing = cursor[segment];
    if (typeof existing !== "object" || existing === null) {
      cursor[segment] = {};
    }
    cursor = cursor[segment] as Record<string, unknown>;
  }
  const leaf = String(path[path.length - 1]);
  if (cursor[leaf] === undefined) {
    cursor[leaf] = error;
  }
}
