import { invoke } from '@tauri-apps/api/core';

export interface DatabaseColumn {
  name: string;
  dataType: string;
  isPrimaryKey: boolean;
  isUnique: boolean;
  isNullable: boolean;
  defaultValue: string | null;
  referencesTable: string | null;
  referencesColumn: string | null;
}

export interface DatabaseTable {
  name: string;
  columns: DatabaseColumn[];
  primaryKeys: string[];
  indexes: string[];
}

export interface DatabaseRelationship {
  fromTable: string;
  fromColumn: string;
  toTable: string;
  toColumn: string;
  kind: string;
}

export interface DatabaseSchema {
  provider: string;
  source: string;
  sources: string[];
  tables: DatabaseTable[];
  relationships: DatabaseRelationship[];
}

export async function detectDatabaseStructure(path: string) {
  return invoke<DatabaseSchema | null>('detect_database_structure', { path });
}
