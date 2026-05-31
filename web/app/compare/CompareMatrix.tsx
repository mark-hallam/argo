"use client";

import { Fragment } from "react";

/**
 * Client-rendered version of the 5×16 competitive comparison matrix.
 *
 * Why a client component for a static table?
 *
 * The matrix is a deeply nested JSX tree: 16 rows × 5 columns ≈ 80
 * score spans + 16 question cells + 5 group headers, plus the table
 * frame. As a *server* component each of those nodes ends up as a
 * separate entry in the React Flight payload that ships alongside
 * the SSR'd HTML (see `<script>self.__next_f.push(...)` blocks). On
 * the dev server that drove the original 321 KB measurement, the
 * Flight payload alone was ~280 KB.
 *
 * As a client component the Flight payload only needs to carry the
 * component boundary + props (the dimensions array + competitors
 * tuple as JSON ≈ 4 KB). The SSR'd HTML still contains the rendered
 * table — React renders client components on the server too — so SEO,
 * no-JS rendering, and Lighthouse "rendered content" all stay intact.
 *
 * The component is otherwise pure presentation: no hooks, no state,
 * no event handlers. The "use client" boundary exists purely to shift
 * the matrix out of the Flight payload.
 */

export type Score = "yes" | "no" | "partial";

export interface Dimension {
  group: string;
  question: string;
  scores: Record<string, Score>;
  caption?: string;
}

interface CompareMatrixProps {
  competitors: readonly string[];
  dimensions: Dimension[];
}

function groupDimensions(dims: Dimension[]) {
  const groups: { name: string; dims: Dimension[] }[] = [];
  for (const d of dims) {
    const last = groups[groups.length - 1];
    if (last && last.name === d.group) {
      last.dims.push(d);
    } else {
      groups.push({ name: d.group, dims: [d] });
    }
  }
  return groups;
}

function scoreClass(score: Score): string {
  return `compare-score ${score}`;
}

function scoreLabel(score: Score): string {
  if (score === "yes") return "Yes";
  if (score === "no") return "No";
  return "Partial";
}

export function CompareMatrix({ competitors, dimensions }: CompareMatrixProps) {
  const groups = groupDimensions(dimensions);
  return (
    <div className="compare-table-wrap">
      <table className="compare-table">
        <caption className="sr-only">
          SKYCOT competitor comparison by trust, audit, approvals, editing,
          enterprise controls, and deployment model.
        </caption>
        <thead>
          <tr>
            <th scope="col">Capability</th>
            {competitors.map((c, idx) => (
              <th key={c} scope="col" className={idx === 0 ? "us-col" : ""}>
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {groups.map((g) => (
            <Fragment key={g.name}>
              <tr className="compare-group-row">
                <th scope="rowgroup">{g.name}</th>
                {competitors.map((c) => (
                  <td key={c} />
                ))}
              </tr>
              {g.dims.map((d, i) => (
                <tr key={`${g.name}-${i}`}>
                  <th scope="row" className="compare-question-cell">
                    {d.question}
                    {d.caption ? <div className="compare-caption">{d.caption}</div> : null}
                  </th>
                  {competitors.map((c, idx) => (
                    <td key={c} className={idx === 0 ? "us-col" : ""}>
                      <span
                        className={scoreClass(d.scores[c])}
                        role="img"
                        aria-label={scoreLabel(d.scores[c])}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </Fragment>
          ))}
          <tr className="compare-footer-row">
            <td colSpan={competitors.length + 1}>
              <strong>If 10+ of those rows landed for you, this is your team.</strong>
              <br />
              <a className="compare-cta" href="/auth/signup">
                Hire your first teammate →
              </a>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
