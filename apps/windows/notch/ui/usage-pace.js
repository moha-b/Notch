'use strict';

globalThis.NotchUsagePace = function (window, now) {
  // Missing, expired, negative, non-finite and count-only readings have no pace.
  const {used, duration_seconds: duration, resets_at: reset, count} = window;
  if (count != null || !Number.isFinite(used) || used < 0 ||
      !Number.isFinite(duration) || duration <= 0 || !Number.isFinite(reset)) return null;
  const remaining = (reset - now) / 1000;
  if (!Number.isFinite(remaining) || remaining <= 0) return null;
  const elapsed = 1 - Math.min(remaining / duration, 1);
  const points = (Math.min(used, 1) - elapsed) * 100;
  const magnitude = Math.abs(points);
  const rounded = Math.round(magnitude * 10) / 10;
  const amount = rounded === 0 && magnitude > 0 ? '<0.1' : String(rounded);
  return {points, deficit: points > 0, summary: `${amount}% ${points > 0 ? 'deficit' : 'reserved'}`};
};
