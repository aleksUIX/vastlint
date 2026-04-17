// Re-export the types from the vastlint npm package so they can be referenced
// without depending on the package being installed during TS compilation.

export interface Issue {
  id: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  path: string | null;
  spec_ref: string;
  line: number | null;
  col: number | null;
}

export interface Summary {
  errors: number;
  warnings: number;
  infos: number;
  valid: boolean;
}

export interface ValidationResult {
  version: string | null;
  issues: Issue[];
  summary: Summary;
}

export interface ValidateOptions {
  wrapper_depth?: number;
  max_wrapper_depth?: number;
  rule_overrides?: Record<string, 'error' | 'warning' | 'info' | 'off'>;
}
