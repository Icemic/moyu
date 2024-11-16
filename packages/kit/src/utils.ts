export function omitBy<T extends Record<string, any>>(
  object: T,
  predicate: (value: T[keyof T], key: keyof T) => boolean,
): Partial<T> {
  const result: Partial<T> = {};
  for (const key of Object.keys(object) as (keyof T)[]) {
    if (!predicate(object[key], key)) {
      result[key] = object[key];
    }
  }
  return result;
}
