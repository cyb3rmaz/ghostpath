import React, { useMemo } from 'react';
import { TrafficPoint } from '../types/vpn';

interface ThroughputChartProps {
  data: TrafficPoint[];
  isConnected: boolean;
}

export const ThroughputChart: React.FC<ThroughputChartProps> = ({ data, isConnected }) => {
  const width = 560;
  const height = 132;
  const padTop = 12;
  const padBottom = 8;
  const plotHeight = height - padTop - padBottom;

  const maxMbps = useMemo(() => {
    const peak = Math.max(
      20,
      ...data.map((d) => Math.max(d.downloadMbps, d.uploadMbps))
    );
    return Math.ceil(peak / 20) * 20;
  }, [data]);

  const { dlLine, dlArea, ulLine } = useMemo(() => {
    if (data.length === 0) {
      return { dlLine: '', dlArea: '', ulLine: '' };
    }
    const stepX = width / Math.max(1, data.length - 1);

    const toPoint = (val: number, index: number) => {
      const x = index * stepX;
      const ratio = Math.min(1, Math.max(0, val / maxMbps));
      const y = padTop + plotHeight - ratio * plotHeight;
      return { x, y };
    };

    const dlPts = data.map((d, i) => toPoint(d.downloadMbps, i));
    const ulPts = data.map((d, i) => toPoint(d.uploadMbps, i));

    const buildSmoothPath = (pts: { x: number; y: number }[]) => {
      if (pts.length === 0) return '';
      if (pts.length === 1) return `M ${pts[0].x} ${pts[0].y}`;
      let d = `M ${pts[0].x.toFixed(1)} ${pts[0].y.toFixed(1)}`;
      for (let i = 0; i < pts.length - 1; i++) {
        const p0 = pts[i];
        const p1 = pts[i + 1];
        const midX = (p0.x + p1.x) / 2;
        d += ` C ${midX.toFixed(1)} ${p0.y.toFixed(1)}, ${midX.toFixed(1)} ${p1.y.toFixed(1)}, ${p1.x.toFixed(1)} ${p1.y.toFixed(1)}`;
      }
      return d;
    };

    const dlPath = buildSmoothPath(dlPts);
    const ulPath = buildSmoothPath(ulPts);
    const last = dlPts[dlPts.length - 1];
    const first = dlPts[0];
    const bottomY = padTop + plotHeight;
    const areaPath = `${dlPath} L ${last.x.toFixed(1)} ${bottomY} L ${first.x.toFixed(1)} ${bottomY} Z`;

    return { dlLine: dlPath, dlArea: areaPath, ulLine: ulPath };
  }, [data, maxMbps, plotHeight]);

  return (
    <div className="relative w-full select-none">
      <div className="flex items-center justify-between text-[11px] font-mono tabular-nums text-[#767E89] mb-1.5">
        <span>0 Mbps</span>
        <span>{(maxMbps / 2).toFixed(0)} Mbps</span>
        <span>{maxMbps.toFixed(0)} Mbps peak</span>
      </div>
      <div className="relative w-full h-[132px] rounded-lg bg-[#131518] border border-[#272A30] overflow-hidden">
        <svg
          viewBox={`0 0 ${width} ${height}`}
          preserveAspectRatio="none"
          className="w-full h-full block"
          aria-label="Live download and upload throughput chart"
        >
          <defs>
            <linearGradient id="ghostpathDlGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#22C55E" stopOpacity={isConnected ? 0.28 : 0.05} />
              <stop offset="100%" stopColor="#22C55E" stopOpacity={0} />
            </linearGradient>
          </defs>

          {/* Horizontal Grid Lines */}
          {[0.25, 0.5, 0.75].map((fraction) => {
            const y = padTop + plotHeight * fraction;
            return (
              <line
                key={fraction}
                x1={0}
                y1={y}
                x2={width}
                y2={y}
                stroke="#24272D"
                strokeWidth={1}
                strokeDasharray="3 4"
              />
            );
          })}

          {/* Vertical Grid Lines */}
          {[0.2, 0.4, 0.6, 0.8].map((fraction) => {
            const x = width * fraction;
            return (
              <line
                key={fraction}
                x1={x}
                y1={0}
                x2={x}
                y2={height}
                stroke="#22252A"
                strokeWidth={1}
              />
            );
          })}

          {/* Download Area & Line */}
          {dlArea && <path d={dlArea} fill="url(#ghostpathDlGradient)" />}
          {ulLine && (
            <path
              d={ulLine}
              fill="none"
              stroke={isConnected ? '#858D98' : '#363C46'}
              strokeWidth={1.5}
              strokeDasharray="4 3"
              strokeLinecap="round"
            />
          )}
          {dlLine && (
            <path
              d={dlLine}
              fill="none"
              stroke={isConnected ? '#22C55E' : '#3E4450'}
              strokeWidth={2}
              strokeLinecap="round"
            />
          )}
        </svg>

        {!isConnected && (
          <div className="absolute inset-0 flex items-center justify-center bg-[#111315]/45">
            <span className="text-xs font-mono text-[#7E8691]">
              Tunnel idle — connect to stream interface throughput
            </span>
          </div>
        )}
      </div>
    </div>
  );
};
