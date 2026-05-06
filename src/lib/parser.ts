import { invoke } from '@tauri-apps/api/core';

export type SourceLanguage =
  | 'javaScript'
  | 'typeScript'
  | 'tsx'
  | 'jsx'
  | 'rust'
  | 'go'
  | 'python'
  | 'java'
  | 'kotlin'
  | 'swift'
  | 'ruby'
  | 'php'
  | 'c'
  | 'cpp'
  | 'cSharp'
  | 'shell'
  | 'sql'
  | 'json'
  | 'yaml'
  | 'toml'
  | 'ini'
  | 'xml'
  | 'css'
  | 'scss'
  | 'less'
  | 'vue'
  | 'svelte'
  | 'astro';

export type SymbolKind =
  | 'function'
  | 'component'
  | 'class'
  | 'method'
  | 'object'
  | 'enum'
  | 'interface'
  | 'typeAlias'
  | 'import'
  | 'export';

export interface SourceRange {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface CodeSymbol {
  name: string;
  kind: SymbolKind;
  parent: string | null;
  source: string | null;
  range: SourceRange;
}

export interface ParsedFile {
  path: string;
  language: SourceLanguage;
  symbols: CodeSymbol[];
}

export async function parseSourceFile(path: string) {
  return invoke<ParsedFile>('parse_source_file', { path });
}
