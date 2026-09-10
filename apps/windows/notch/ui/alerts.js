globalThis.NotchAlerts = class NotchAlerts {
  constructor() {
    this.crossed = new Map();
    this.activity = new Map();
  }

  observe(providers, settings, states) {
    const alerts = [];
    const active = new Set(providers.map(provider => provider.id));
    for (const id of this.activity.keys()) if (!active.has(id)) this.activity.delete(id);
    for (const provider of providers) {
      const muted = settings.muted_providers?.includes(provider.id);
      const thresholds = this.thresholds(provider);
      const session = this.session(provider, states[provider.id]);
      if (muted) continue;
      alerts.push(...thresholds);
      if (session && (settings.announce_completion || settings.sounds)) alerts.push(session);
    }
    return alerts;
  }

  thresholds(provider) {
    if (provider.snap.status !== 'ok') return [];
    const windows = provider.snap.windows.filter(window => window.count == null);
    if (!windows.length) return [];
    const headline = windows.reduce((left, right) => left.used >= right.used ? left : right);
    const level = headline.used >= 1 ? 100 : headline.used >= 0.8 ? 80 : 0;
    const previous = this.crossed.get(provider.id) || 0;
    this.crossed.set(provider.id, level);
    return [80, 100].filter(threshold => threshold > previous && threshold <= level).map(threshold => ({
      provider: provider.id, kind: 'threshold',
      message: threshold === 100 ? `${provider.name} limit reached` : `${provider.name} is at ${Math.round(headline.used * 100)}%`,
    }));
  }

  session(provider, state) {
    const previous = this.activity.get(provider.id);
    this.activity.set(provider.id, state);
    if (previous == null || previous === state) return null;
    if (state === 'attention') return {provider: provider.id, kind: 'session', message: `${provider.name} needs attention`};
    if (previous === 'running' && ['idle', 'done'].includes(state)) {
      return {provider: provider.id, kind: 'session', message: `${provider.name} finished`};
    }
    return null;
  }
};
