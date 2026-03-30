export interface Dependency {
  name: string;
  desc: string;
  check: () => Promise<boolean>;
  install: () => Promise<string>;
}
