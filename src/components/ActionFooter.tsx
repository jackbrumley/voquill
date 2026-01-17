
import { ComponentChildren } from 'preact';

interface ActionFooterProps {
  children: ComponentChildren;
}

export const ActionFooter = ({ children }: ActionFooterProps) => {
  return (
    <div className="form-actions-bottom">
      {children}
    </div>
  );
};
