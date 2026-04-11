
import { ComponentChildren } from 'preact';
import { tokens } from '../design-tokens.ts';

interface ActionFooterProps {
  children: ComponentChildren;
}

export const ActionFooter = ({ children }: ActionFooterProps) => {
  return (
    <div
      style={{
        position: 'absolute',
        bottom: tokens.spacing.lg,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 100,
        display: 'flex',
        justifyContent: 'center',
        transition: tokens.transitions.normal,
        pointerEvents: 'none',
        width: '100%',
      }}
    >
      {children}
    </div>
  );
};
