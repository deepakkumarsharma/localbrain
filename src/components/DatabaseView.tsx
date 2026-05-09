import { useMemo, useState } from 'react';
import type { DatabaseSchema, DatabaseTable } from '../lib/database';

interface DatabaseViewProps {
  schema: DatabaseSchema | null;
}

export function DatabaseView({ schema }: DatabaseViewProps) {
  const [selectedTable, setSelectedTable] = useState<string>('');
  const [search, setSearch] = useState('');

  const tables = useMemo(() => schema?.tables ?? [], [schema]);
  const relationships = useMemo(() => schema?.relationships ?? [], [schema]);
  const filteredTables = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) {
      return tables;
    }
    return tables.filter((table) => table.name.toLowerCase().includes(query));
  }, [search, tables]);
  const resolvedSelectedTable = filteredTables.length
    ? (filteredTables.find((table) => table.name === selectedTable) ?? filteredTables[0])
    : null;

  if (!schema) {
    return (
      <div className="absolute inset-0 overflow-auto bg-app-background">
        <div className="mx-auto max-w-[1120px] px-8 py-10">
          <div className="rounded-xl border border-app-border bg-app-panel p-5 text-sm text-app-muted">
            No database schema detected in this workspace.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="absolute inset-0 overflow-auto bg-app-background app-scrollbar">
      <div className="mx-auto max-w-[1500px] px-6 py-6">
        <div className="mb-4 rounded-xl border border-app-border bg-app-panel p-4">
          <div className="mb-2 text-[11px] font-black uppercase tracking-widest text-app-muted">
            Database Structure
          </div>
          <div className="grid grid-cols-1 gap-2 md:grid-cols-4">
            <StatCard label="Provider" value={schema.provider} />
            <StatCard label="Tables" value={String(tables.length)} />
            <StatCard label="Relationships" value={String(relationships.length)} />
            <StatCard label="Schema Files" value={String(schema.sources.length)} />
          </div>
          <div className="mt-3 rounded-lg border border-app-border bg-app-background px-3 py-2 text-xs text-app-muted">
            Root Source: <span className="font-mono text-app-text">{schema.source}</span>
          </div>
        </div>

        <div className="grid min-h-[640px] grid-cols-1 gap-4 xl:grid-cols-[320px_1fr_520px]">
          <section className="rounded-xl border border-app-border bg-app-panel p-3">
            <div className="mb-2 flex items-center justify-between">
              <div className="text-[11px] font-black uppercase tracking-widest text-app-muted">
                Tables
              </div>
              <div className="text-xs font-bold text-app-muted">{filteredTables.length} shown</div>
            </div>
            <input
              className="mb-3 w-full rounded-lg border border-app-border bg-app-background px-2.5 py-2 text-sm text-app-text outline-none focus:border-app-accent"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search table..."
            />
            <div className="max-h-[520px] space-y-1 overflow-auto app-scrollbar pr-1">
              {filteredTables.map((table) => (
                <button
                  key={table.name}
                  className="w-full rounded-md border border-app-border bg-app-background px-2.5 py-2 text-left hover:border-app-accent data-[active=true]:border-app-accent data-[active=true]:bg-app-accentSoft/30"
                  type="button"
                  data-active={resolvedSelectedTable?.name === table.name}
                  onClick={() => setSelectedTable(table.name)}
                >
                  <div className="truncate text-sm font-bold text-app-text">{table.name}</div>
                  <div className="mt-0.5 text-[11px] text-app-muted">
                    {table.columns.length} cols · {table.indexes.length} idx
                  </div>
                </button>
              ))}
            </div>
          </section>

          <section className="rounded-xl border border-app-border bg-app-panel p-4">
            <div className="mb-2 text-[11px] font-black uppercase tracking-widest text-app-muted">
              Relationships (ER)
            </div>
            {relationships.length === 0 ? (
              <p className="text-sm text-app-muted">No foreign-key relationships found.</p>
            ) : (
              <div className="max-h-[580px] space-y-2 overflow-auto app-scrollbar pr-1">
                {relationships.map((relationship, index) => (
                  <div
                    key={`${relationship.fromTable}-${relationship.fromColumn}-${relationship.toTable}-${relationship.toColumn}-${index}`}
                    className="rounded-lg border border-app-border bg-app-background p-2.5 text-sm"
                  >
                    <div className="font-mono text-app-text">
                      {relationship.fromTable}.{relationship.fromColumn}
                    </div>
                    <div className="text-xs text-app-muted">{relationship.kind}</div>
                    <div className="font-mono text-app-text">
                      {relationship.toTable}.{relationship.toColumn}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>

          <section className="rounded-xl border border-app-border bg-app-panel p-4">
            <div className="mb-2 text-[11px] font-black uppercase tracking-widest text-app-muted">
              Table Details
            </div>
            {resolvedSelectedTable ? (
              <TableDetails table={resolvedSelectedTable} />
            ) : (
              <p className="text-sm text-app-muted">Select a table to inspect columns.</p>
            )}
          </section>
        </div>

        {schema.sources.length > 0 ? (
          <section className="mt-4 rounded-xl border border-app-border bg-app-panel p-4">
            <div className="mb-2 text-[11px] font-black uppercase tracking-widest text-app-muted">
              Detected Schema Files
            </div>
            <div className="max-h-40 space-y-1 overflow-auto app-scrollbar pr-1">
              {schema.sources.map((source) => (
                <div
                  key={source}
                  className="truncate rounded-md border border-app-border bg-app-background px-2.5 py-1.5 font-mono text-xs text-app-muted"
                  title={source}
                >
                  {source}
                </div>
              ))}
            </div>
          </section>
        ) : null}
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-app-border bg-app-background px-3 py-2">
      <div className="text-[10px] font-black uppercase tracking-widest text-app-muted">{label}</div>
      <div className="mt-0.5 truncate text-sm font-bold text-app-text">{value}</div>
    </div>
  );
}

function TableDetails({ table }: { table: DatabaseTable }) {
  const generatedType = useMemo(() => {
    const fields = table.columns.map((column) => {
      const type = toTypeScriptType(column.dataType, column.isNullable);
      return `  ${column.name}: ${type};`;
    });
    return `export interface ${table.name} {\n${fields.join('\n')}\n}`;
  }, [table]);

  return (
    <div>
      <div className="mb-2 text-sm font-bold text-app-text">{table.name}</div>
      <div className="mb-3 max-h-[380px] overflow-auto rounded-lg border border-app-border bg-app-background app-scrollbar">
        <table className="min-w-full text-left text-xs">
          <thead className="bg-app-panelSoft text-app-muted">
            <tr>
              <th className="px-2 py-1.5">Column</th>
              <th className="px-2 py-1.5">Type</th>
              <th className="px-2 py-1.5">Constraints</th>
            </tr>
          </thead>
          <tbody>
            {table.columns.map((column) => {
              const constraints = [
                column.isPrimaryKey ? 'PK' : null,
                column.isUnique ? 'UNIQUE' : null,
                !column.isNullable ? 'NOT NULL' : null,
                column.defaultValue ? `DEFAULT ${column.defaultValue}` : null,
                column.referencesTable && column.referencesColumn
                  ? `FK -> ${column.referencesTable}.${column.referencesColumn}`
                  : null,
              ]
                .filter(Boolean)
                .join(', ');

              return (
                <tr key={column.name} className="border-t border-app-border">
                  <td className="px-2 py-1.5 font-mono text-app-text">{column.name}</td>
                  <td className="px-2 py-1.5 font-mono text-app-text">{column.dataType}</td>
                  <td className="px-2 py-1.5 text-app-muted">{constraints || '-'}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="text-[11px] font-black uppercase tracking-widest text-app-muted">
        Generated TypeScript
      </div>
      <pre className="mt-2 max-h-[220px] overflow-auto rounded-lg border border-app-border bg-app-background p-3 text-xs text-app-text app-scrollbar">
        <code>{generatedType}</code>
      </pre>
    </div>
  );
}

function toTypeScriptType(raw: string, nullable: boolean) {
  const normalized = raw.toLowerCase();
  let base = 'unknown';
  if (normalized.includes('int') || normalized === 'bigint') base = 'number';
  else if (
    normalized.includes('float') ||
    normalized.includes('double') ||
    normalized.includes('decimal')
  )
    base = 'number';
  else if (normalized.includes('bool')) base = 'boolean';
  else if (normalized.includes('date') || normalized.includes('time')) base = 'Date';
  else if (normalized.includes('json')) base = 'Record<string, unknown>';
  else if (
    normalized.includes('string') ||
    normalized.includes('char') ||
    normalized.includes('text')
  )
    base = 'string';

  return nullable ? `${base} | null` : base;
}
