
export const tokens = {
  colors: {
    // Backgrounds
    bgPrimary: '#36393f',
    bgSecondary: '#2f3136',
    bgTertiary: '#202225',
    bgHover: '#40444b',
    bgGradientWarm: '#4a384d',
    bgGradientCool: '#2d454f',
    
    // Text
    textPrimary: '#ffffff',
    textSecondary: '#8e9297',
    textMuted: '#72767d',
    
    // Brand/Action
    accentPrimary: '#5865f2',
    accentHover: '#4752c4',
    success: '#10b981',
    error: '#ef4444',
  },
  
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
  },
  
  radii: {
    input: '8px',
    panel: '12px',
    button: '8px',
  },
  
  shadows: {
    sm: '0 2px 8px rgba(0, 0, 0, 0.2)',
    md: '0 6px 16px rgba(0, 0, 0, 0.3)',
    lg: '0 12px 32px rgba(0, 0, 0, 0.4)',
    accent: '0 4px 12px rgba(88, 101, 242, 0.4)',
  },
  
  transitions: {
    fast: 'all 0.15s cubic-bezier(0.4, 0, 0.2, 1)',
    normal: 'all 0.25s cubic-bezier(0.4, 0, 0.2, 1)',
    slow: 'all 0.4s cubic-bezier(0.4, 0, 0.2, 1)',
  },
  
  typography: {
    fontMain: "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Helvetica Neue', Arial, sans-serif",
    fontMono: "'Consolas', 'Monaco', 'Courier New', monospace",
    sizeXs: '11px',
    sizeSm: '13px',
    sizeMd: '14px',
    sizeLg: '16px',
    sizeXl: '18px',
    sizeHuge: '32px',
  }
} as const;

export type DesignTokens = typeof tokens;

// Helper to convert camelCase to kebab-case for CSS variables
export const tokensToCssVars = (obj: any, prefix = '--'): Record<string, string> => {
  const vars: Record<string, string> = {};
  
  const iterate = (current: any, currentPrefix: string) => {
    for (const key in current) {
      const value = current[key];
      const kebabKey = key.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
      const newPrefix = `${currentPrefix}${kebabKey}`;
      
      if (typeof value === 'object' && value !== null) {
        iterate(value, `${newPrefix}-`);
      } else {
        vars[newPrefix] = value;
      }
    }
  };
  
  iterate(obj, prefix);
  return vars;
};
